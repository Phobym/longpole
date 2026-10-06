//! `settings.json` с языком интерфейса и выбор языка по системной локали.

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::request::ProjectRef;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::{AppSettings, Settings, SettingsPatch, Theme};
use tempfile::TempDir;

fn settings() -> (TempDir, Settings) {
    let dir = TempDir::new().expect("tempdir");
    let settings = Settings::new(dir.path().join("data").join("settings.json"));
    (dir, settings)
}

#[test]
fn системная_локаль_ru_даёт_русский_остальное_английский() {
    for tag in ["ru", "ru-RU", "RU_ru", "ru-UA"] {
        assert_eq!(Locale::from_tag(tag), Locale::Ru, "{tag}");
    }
    for tag in ["en-US", "de-DE", "uk-UA", ""] {
        assert_eq!(Locale::from_tag(tag), Locale::En, "{tag:?}");
    }
}

#[test]
fn без_сохранённого_выбора_язык_системный() {
    let (_dir, settings) = settings();
    assert_eq!(settings.locale().unwrap(), Locale::system());
}

#[test]
fn выбор_языка_сохраняется_и_переживает_перезапуск() {
    let (dir, settings) = settings();
    let other = if Locale::system() == Locale::Ru {
        Locale::En
    } else {
        Locale::Ru
    };
    settings
        .update(SettingsPatch {
            locale: Some(other),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(settings.locale().unwrap(), other);

    let again = Settings::new(dir.path().join("data").join("settings.json"));
    assert_eq!(again.locale().unwrap(), other);
    let file = std::fs::read_to_string(dir.path().join("data").join("settings.json")).unwrap();
    assert!(file.contains("\"locale\""));
}

#[test]
fn битый_файл_настроек_это_ошибка_storage() {
    let (dir, settings) = settings();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data").join("settings.json"), "[").unwrap();
    assert_eq!(settings.locale().unwrap_err().code, ErrorCode::Storage);
}

#[test]
fn без_файла_тема_системная_последнего_проекта_нет() {
    let (_dir, settings) = settings();
    let got = settings.get().unwrap();
    assert_eq!(got.theme, Theme::System);
    assert_eq!(got.last_project, None);
    assert_eq!(got.locale, Locale::system());
}

#[test]
fn патч_меняет_только_переданное_и_переживает_перезапуск() {
    let (dir, settings) = settings();
    settings
        .update(SettingsPatch {
            theme: Some(Theme::Dark),
            ..Default::default()
        })
        .unwrap();
    let project = ProjectRef {
        host: "h.example".into(),
        path: "g/p".into(),
    };
    let got = settings
        .update(SettingsPatch {
            last_project: Some(project.clone()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(got.theme, Theme::Dark);
    assert_eq!(got.last_project, Some(project.clone()));

    let again = Settings::new(dir.path().join("data").join("settings.json"));
    assert_eq!(
        again.get().unwrap(),
        AppSettings {
            locale: Locale::system(),
            theme: Theme::Dark,
            last_project: Some(project.clone()),
        }
    );
    again.forget_project(&project).unwrap();
    assert_eq!(again.get().unwrap().last_project, None);
}

#[test]
fn старый_файл_только_с_locale_читается() {
    let (dir, settings) = settings();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(
        dir.path().join("data").join("settings.json"),
        "{\"locale\":\"en\"}",
    )
    .unwrap();
    let got = settings.get().unwrap();
    assert_eq!(
        (got.locale, got.theme, got.last_project),
        (Locale::En, Theme::System, None)
    );
}
