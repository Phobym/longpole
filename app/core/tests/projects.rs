//! «Мои проекты»: `projects.json` на tempdir.

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::projects::{Projects, SavedProject};
use pipeline_trace_core::request::ProjectRef;
use tempfile::TempDir;
use time::macros::datetime;

fn projects() -> (TempDir, Projects) {
    let dir = TempDir::new().expect("tempdir");
    let projects = Projects::new(dir.path().join("data").join("projects.json"));
    (dir, projects)
}

fn p(path: &str) -> ProjectRef {
    ProjectRef {
        host: "h.example".into(),
        path: path.into(),
    }
}

const T0: time::OffsetDateTime = datetime!(2026-10-06 10:00:00 UTC);

#[test]
fn пустой_файл_это_пустой_список() {
    let (_dir, projects) = projects();
    assert_eq!(projects.list().unwrap(), []);
}

#[test]
fn порядок_добавления_повтор_обновляет_имя_на_месте() {
    let (_dir, projects) = projects();
    projects.add(p("g/a"), "A", T0).unwrap();
    projects.add(p("g/b"), "B", T0).unwrap();
    let list = projects.add(p("g/a"), "A2", T0).unwrap();
    assert_eq!(
        list,
        vec![
            SavedProject {
                host: "h.example".into(),
                path: "g/a".into(),
                name: "A2".into(),
                added_at: "2026-10-06T10:00:00.000Z".into(),
            },
            SavedProject {
                host: "h.example".into(),
                path: "g/b".into(),
                name: "B".into(),
                added_at: "2026-10-06T10:00:00.000Z".into(),
            },
        ]
    );
}

#[test]
fn удаление_и_перезапуск() {
    let (dir, projects) = projects();
    projects.add(p("g/a"), "A", T0).unwrap();
    projects.add(p("g/b"), "B", T0).unwrap();
    assert_eq!(projects.remove(&p("g/a")).unwrap().len(), 1);
    let again = Projects::new(dir.path().join("data").join("projects.json"));
    let list = again.list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].path, "g/b");
    let file = std::fs::read_to_string(dir.path().join("data").join("projects.json")).unwrap();
    assert!(file.contains("\"addedAt\""));
}

#[test]
fn битый_файл_это_ошибка_storage() {
    let (dir, projects) = projects();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("projects.json"), "{").unwrap();
    assert_eq!(projects.list().unwrap_err().code, ErrorCode::Storage);
}
