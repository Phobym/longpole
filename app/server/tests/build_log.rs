//! Лог построения (спека поставки, § 13). Отдельный бинарник — по той же причине, что `request_log.rs`.

mod fixtures;
mod ghes_run;

use std::io::Write;
use std::sync::{Arc, Mutex};

use fixtures::*;
use ghes_run::*;

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
async fn build_log_has_key_cache_hit_and_duration() {
    let log = Captured::default();
    let writer = log.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let mock = providers(28800).await;
    mount_run(&mock, "completed", "2026-10-07T10:05:00Z").await;
    let (app, cookie) = signed_in(&mock).await;

    for _ in 0..2 {
        last(&app, build(&cookie, &run_url(), "en", false)).await;
    }

    let log = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
    let builds: Vec<_> = log.lines().filter(|l| l.contains(" build ")).collect();
    assert_eq!(builds.len(), 2, "{log}");
    for (line, cached) in builds.iter().zip(["cached=false", "cached=true"]) {
        assert!(line.contains(&format!("host={GHES}")), "{line}");
        assert!(line.contains("project=42"), "{line}");
        assert!(line.contains("pipeline=7"), "{line}");
        assert!(line.contains(cached), "{line}");
        assert!(line.contains("ms="), "{line}");
    }
}
