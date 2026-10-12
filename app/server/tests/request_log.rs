//! Отдельный бинарник: подписчик `tracing` на поток ненадёжен, пока параллельные тесты того же процесса
//! кэшируют interest колсайтов без подписчика.

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::Request;
use longpole_server::app;
use longpole_server::config::Config;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn request_log_has_method_path_status_without_query() {
    let log = Captured::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let yaml = "url: https://longpole.example.com\ncookie_key: MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=\n\
                hosts:\n  - {domain: gitlab.example.com, provider: gitlab, client_id: id, client_secret: s}\n";
    let request =
        Request::get("/auth/callback/gitlab.example.com?code=SECRET-CODE&state=SECRET-STATE")
            .body(Body::empty())
            .unwrap();

    app(&Config::parse(yaml, |_| None).unwrap())
        .oneshot(request)
        .await
        .unwrap();

    let log = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("method=GET"), "{log}");
    assert!(
        log.contains("path=/auth/callback/gitlab.example.com "),
        "{log}"
    );
    assert!(log.contains("status=404"), "{log}");
    assert!(log.contains("ms="), "{log}");
    assert!(!log.contains("SECRET"), "{log}");
}
