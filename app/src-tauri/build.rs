use std::path::Path;

/// Команды формы: на каждую `allow-<имя>` в `capabilities/form.json`. Без манифеста свои команды
/// доступны любому локальному окну, включая `report://`; список должен совпадать с `generate_handler!`.
const COMMANDS: &[&str] = &[
    "hosts",
    "set_token",
    "remove_token",
    "history",
    "remove_history",
    "clear_history",
    "projects",
    "branches",
    "pipelines",
    "build",
    "get_locale",
    "set_locale",
];

/// Собранный шаблон отчёта (`npm run build`): `include_str!` в `reports.rs` читает его.
const REPORT_TEMPLATE: &str = "../dist-report/report.html";

fn main() {
    if !Path::new(REPORT_TEMPLATE).is_file() {
        panic!("нет {REPORT_TEMPLATE}: сначала `npm run build` в app/");
    }
    println!("cargo:rerun-if-changed={REPORT_TEMPLATE}");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("tauri-build");
}
