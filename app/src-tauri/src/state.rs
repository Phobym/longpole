//! Общее состояние приложения: хранилища на диске, отчёты в памяти, масштаб окна.

use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use pipeline_trace_core::history::History;
use pipeline_trace_core::projects::Projects;
use pipeline_trace_core::request::Request;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::Settings;
use pipeline_trace_core::tokens::{TokenStore, platform_store};

const ZOOM_MIN: f64 = 0.25;
const ZOOM_MAX: f64 = 5.0;
/// Сколько отчётов держится в памяти: повтор из «Недавних» не ходит в GitLab, старые вытесняются.
// ponytail: html и json каждого отчёта лежат целиком; если память станет заметна — хранить только json и рендерить html при ⌘S
const REPORTS_KEPT: usize = 10;

/// Отчёт в памяти: `json` отдаёт команда `report`, `html` пишет «Сохранить отчёт…».
pub struct ReportEntry {
    pub host: String,
    pub request: Request,
    pub json: String,
    pub html: String,
    pub file_name: String,
}

pub struct AppState {
    pub tokens: TokenStore,
    pub history: History,
    pub settings: Settings,
    pub projects: Projects,
    /// от старого к новому; id растут
    reports: Mutex<VecDeque<(u32, ReportEntry)>>,
    /// отчёт на экране формы: ему адресовано ⌘S
    current_report: Mutex<Option<u32>>,
    /// label окна → масштаб: у webview нет геттера
    pub zoom: Mutex<HashMap<String, f64>>,
    next_report: AtomicU32,
}

impl AppState {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            tokens: TokenStore::new(data_dir.join("hosts.json"), platform_store()),
            history: History::new(data_dir.join("history.json")),
            settings: Settings::new(data_dir.join("settings.json")),
            projects: Projects::new(data_dir.join("projects.json")),
            reports: Mutex::default(),
            current_report: Mutex::default(),
            zoom: Mutex::default(),
            next_report: AtomicU32::new(0),
        }
    }

    /// Новый отчёт получает id; самый старый сверх `REPORTS_KEPT` вытесняется.
    pub fn push_report(&self, entry: ReportEntry) -> u32 {
        let id = self.next_report.fetch_add(1, Ordering::SeqCst) + 1;
        let mut reports = lock(&self.reports);
        reports.push_back((id, entry));
        while reports.len() > REPORTS_KEPT {
            reports.pop_front();
        }
        id
    }

    pub fn report_json(&self, id: u32) -> Option<String> {
        lock(&self.reports)
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, entry)| entry.json.clone())
    }

    /// Свежайший отчёт с тем же хостом и запросом — ключ записи истории.
    pub fn find_report(&self, host: &str, request: &Request) -> Option<u32> {
        lock(&self.reports)
            .iter()
            .rev()
            .find(|(_, e)| e.host == host && e.request == *request)
            .map(|(id, _)| *id)
    }

    pub fn set_current(&self, id: Option<u32>) {
        *lock(&self.current_report) = id;
    }

    /// `html` и имя файла отчёта на экране.
    pub fn current_report(&self) -> Option<(String, String)> {
        let id = (*lock(&self.current_report))?;
        lock(&self.reports)
            .iter()
            .find(|(found, _)| *found == id)
            .map(|(_, e)| (e.html.clone(), e.file_name.clone()))
    }

    /// Окно закрыто: его масштаб больше не нужен.
    pub fn forget(&self, label: &str) {
        lock(&self.zoom).remove(label);
    }

    /// Новый масштаб окна после `change`, в пределах `ZOOM_MIN..=ZOOM_MAX`.
    pub fn zoom_by(&self, label: &str, change: impl Fn(f64) -> f64) -> f64 {
        let mut zooms = lock(&self.zoom);
        let factor = zooms.entry(label.into()).or_insert(1.0);
        *factor = change(*factor).clamp(ZOOM_MIN, ZOOM_MAX);
        *factor
    }

    /// Язык интерфейса; не читается файл настроек — системный.
    pub fn locale(&self) -> Locale {
        self.settings.locale().unwrap_or_else(|_| Locale::system())
    }
}

/// Замки охраняют только карты в памяти: паника в другом потоке не повод падать дальше.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32) -> ReportEntry {
        ReportEntry {
            host: "h".into(),
            request: Request::Pipeline {
                project: "g/p".into(),
                pipeline_id: n.to_string(),
            },
            json: format!("{{\"n\":{n}}}"),
            html: String::new(),
            file_name: format!("{n}.html"),
        }
    }

    #[test]
    fn одиннадцатый_отчёт_вытесняет_первый_а_поиск_находит_свежайший() {
        let state = AppState::new(&std::env::temp_dir().join("pipeline-trace-state-test"));
        let ids: Vec<u32> = (1..=11).map(|n| state.push_report(entry(n))).collect();
        assert_eq!(ids[0], 1);
        assert_eq!(state.report_json(1), None);
        assert_eq!(state.report_json(2).as_deref(), Some("{\"n\":2}"));
        let same = state.push_report(entry(2));
        assert_eq!(state.find_report("h", &entry(2).request), Some(same));
        assert_eq!(state.find_report("h", &entry(1).request), None);
        state.set_current(Some(same));
        assert_eq!(
            state.current_report().map(|(_, f)| f).as_deref(),
            Some("2.html")
        );
        state.set_current(None);
        assert_eq!(state.current_report(), None);
    }
}
