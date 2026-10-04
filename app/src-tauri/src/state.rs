//! Общее состояние приложения: хранилища на диске и то, что оболочка помнит про окна.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicU32;
use std::sync::{Mutex, MutexGuard, PoisonError};

use pipeline_trace_core::history::History;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::Settings;
use pipeline_trace_core::tokens::{TokenStore, platform_store};

/// Отчёт в памяти: окно `report-<n>` получает `html` по схеме `report://`, «Сохранить» пишет его же.
#[derive(Clone)]
pub struct ReportEntry {
    pub html: String,
    pub file_name: String,
}

pub struct AppState {
    pub tokens: TokenStore,
    pub history: History,
    pub settings: Settings,
    /// label окна → его отчёт; запись уходит вместе с окном
    pub reports: Mutex<HashMap<String, ReportEntry>>,
    /// label окна, которое получило фокус последним: ему адресованы ⌘S и пункты «Вид»
    pub last_focused: Mutex<Option<String>>,
    /// label окна → масштаб: у webview нет геттера
    pub zoom: Mutex<HashMap<String, f64>>,
    pub next_report: AtomicU32,
}

impl AppState {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            tokens: TokenStore::new(data_dir.join("hosts.json"), platform_store()),
            history: History::new(data_dir.join("history.json")),
            settings: Settings::new(data_dir.join("settings.json")),
            reports: Mutex::default(),
            last_focused: Mutex::default(),
            zoom: Mutex::default(),
            next_report: AtomicU32::new(0),
        }
    }

    /// Язык интерфейса; не читается файл настроек — системный.
    pub fn locale(&self) -> Locale {
        self.settings.locale().unwrap_or_else(|_| Locale::system())
    }
}

/// Замки охраняют только карты окон: паника в другом потоке не повод падать дальше.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
