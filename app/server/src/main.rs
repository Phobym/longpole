use std::process::ExitCode;
use std::sync::LazyLock;

use longpole_server::app;
use longpole_server::config::{self, Config, Log, LogFormat, LogLevel};
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tracing::level_filters::LevelFilter;

/// Версия релиза: `release.yml` проставляет её в `package.json`, Cargo-версия крейтов всегда 0.0.0.
static VERSION: LazyLock<String> = LazyLock::new(|| {
    let package: serde_json::Value =
        serde_json::from_str(include_str!("../../package.json")).unwrap();
    package["version"].as_str().unwrap_or_default().to_owned()
});

#[tokio::main]
async fn main() -> ExitCode {
    let config = match load() {
        Ok(config) => config,
        Err(errors) => {
            for error in errors {
                eprintln!("{}: {error}", config::PATH);
            }
            return ExitCode::FAILURE;
        }
    };
    init_logs(&config.log);
    let domains: Vec<_> = config.hosts.iter().map(|h| h.domain.as_str()).collect();
    tracing::info!(version = %*VERSION, url = %config.url, hosts = ?domains, "start");

    let listener = match TcpListener::bind(config.listen).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(listen = %config.listen, "не удалось занять адрес: {e}");
            return ExitCode::FAILURE;
        }
    };
    match axum::serve(listener, app(&config))
        .with_graceful_shutdown(shutdown())
        .await
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("сервер остановился с ошибкой: {e}");
            ExitCode::FAILURE
        }
    }
}

fn load() -> Result<Config, Vec<String>> {
    let text = std::fs::read_to_string(config::PATH)
        .map_err(|e| vec![format!("не прочитать файл: {e}")])?;
    Config::parse(&text, |name| std::env::var(name).ok())
}

/// В stdout; `RUST_LOG` не читается (спека, § 4).
fn init_logs(log: &Log) {
    let level = match log.level {
        LogLevel::Trace => LevelFilter::TRACE,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Error => LevelFilter::ERROR,
    };
    let logs = tracing_subscriber::fmt().with_max_level(level);
    match log.format {
        LogFormat::Text => logs.init(),
        LogFormat::Json => logs.json().init(),
    }
}

/// В контейнере процесс — PID 1: без своего обработчика SIGTERM `docker stop` ждёт SIGKILL.
async fn shutdown() {
    let mut terminate = signal(SignalKind::terminate()).expect("обработчик SIGTERM");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}
