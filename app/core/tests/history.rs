//! История запросов: `desktop/history.test` на tempdir.

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::history::{History, HistoryEntry, HistoryLabel, NewEntry};
use pipeline_trace_core::request::{Form, FormMode, Request};
use serde_json::json;
use tempfile::TempDir;
use time::{Duration, OffsetDateTime, macros::datetime};

const T0: OffsetDateTime = datetime!(2026-09-24 00:00:00 UTC);

fn history() -> (TempDir, History) {
    let dir = TempDir::new().expect("tempdir");
    let history = History::new(dir.path().join("data").join("history.json"));
    (dir, history)
}

fn entry(i: usize) -> NewEntry {
    NewEntry {
        host: "h".into(),
        form: Form {
            mode: FormMode::Link,
            url: format!("u{i}"),
            ..Default::default()
        },
        request: Request::Pipeline {
            project: "g/p".into(),
            pipeline_id: i.to_string(),
        },
        label: HistoryLabel {
            project: "g/p".into(),
            label: Some(format!("#{i}")),
        },
    }
}

fn minute(i: i64) -> OffsetDateTime {
    T0 + Duration::minutes(i)
}

fn labels(list: &[HistoryEntry]) -> Vec<String> {
    list.iter()
        .map(|e| e.label.label.clone().expect("метка"))
        .collect()
}

#[test]
fn пустой_файл_это_пустой_список() {
    let (_dir, history) = history();
    assert_eq!(history.list().unwrap(), []);
}

#[test]
fn новые_сверху_лимит_20_повтор_поднимается_без_дубля() {
    let (_dir, history) = history();
    for i in 1..=21 {
        history.add(entry(i), minute(i as i64)).unwrap();
    }
    let list = history.list().unwrap();
    assert_eq!(list.len(), 20);
    assert_eq!(labels(&list)[0], "#21");
    assert_eq!(labels(&list)[19], "#2");

    let list = history.add(entry(5), minute(30)).unwrap();
    assert_eq!(list.len(), 20);
    assert_eq!(labels(&list)[..3], ["#5", "#21", "#20"]);
    assert_eq!(list[0].at, "2026-09-24T00:30:00.000Z");
    assert_eq!(history.list().unwrap(), list);
}

#[test]
fn дедупликация_по_хосту_и_запросу_а_не_по_форме() {
    let (_dir, history) = history();
    history.add(entry(1), minute(1)).unwrap();
    let mut other_form = entry(1);
    other_form.form.url = "другая запись той же ссылки".into();
    let list = history.add(other_form, minute(2)).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].form.url, "другая запись той же ссылки");

    let mut other_host = entry(1);
    other_host.host = "h2".into();
    assert_eq!(history.add(other_host, minute(3)).unwrap().len(), 2);
}

#[test]
fn remove_убирает_запись_по_at_clear_очищает() {
    let (_dir, history) = history();
    history.add(entry(1), minute(1)).unwrap();
    let list = history.add(entry(2), minute(2)).unwrap();
    let first = list.iter().find(|e| e.label.label.as_deref() == Some("#1"));

    let after = history.remove(&first.unwrap().at).unwrap();
    assert_eq!(labels(&after), ["#2"]);
    assert_eq!(labels(&history.list().unwrap()), ["#2"]);

    history.clear().unwrap();
    assert_eq!(history.list().unwrap(), []);
}

#[test]
fn в_файл_попадают_только_поля_формы_без_токена() {
    let (dir, history) = history();
    let form: Form = serde_json::from_value(json!({
        "mode": "link", "url": "u", "token": "secret", "extra": "field",
    }))
    .unwrap();
    history
        .add(NewEntry { form, ..entry(1) }, minute(1))
        .unwrap();

    let content = std::fs::read_to_string(dir.path().join("data").join("history.json")).unwrap();
    assert!(!content.contains("secret"));
    assert!(!content.contains("extra"));
    assert_eq!(history.list().unwrap()[0].form.url, "u");
}

#[test]
fn query_и_фрагмент_ссылки_в_историю_не_попадают() {
    let (_dir, history) = history();
    let mut new = entry(1);
    new.form.url = "https://h/g/p/-/pipelines/1?private_token=secret#x".into();
    let list = history.add(new, minute(1)).unwrap();
    assert_eq!(list[0].form.url, "https://h/g/p/-/pipelines/1");
    let content = std::fs::read_to_string(_dir.path().join("data").join("history.json")).unwrap();
    assert!(!content.contains("secret"));
}

#[test]
fn запись_не_оставляет_временных_файлов() {
    let (dir, history) = history();
    history.add(entry(1), minute(1)).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path().join("data"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["history.json"]);
}

#[test]
fn история_переживает_перезапуск() {
    let (dir, history) = history();
    history.add(entry(1), minute(1)).unwrap();
    let again = History::new(dir.path().join("data").join("history.json"));
    assert_eq!(labels(&again.list().unwrap()), ["#1"]);
}

#[test]
fn битый_файл_это_ошибка_storage_а_не_пустая_история() {
    let (dir, history) = history();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("history.json"), "{не json").unwrap();
    assert_eq!(history.list().unwrap_err().code, ErrorCode::Storage);
}
