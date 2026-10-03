//! `settings.json` с языком интерфейса и выбор языка по системной локали.

use pipeline_trace_core::error::ErrorCode;
use pipeline_trace_core::schema::Locale;
use pipeline_trace_core::settings::Settings;
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
    settings.set_locale(other).unwrap();
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
