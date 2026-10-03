fn main() {
    // Без манифеста свои команды доступны любому локальному окну, включая report://.
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[])),
    )
    .expect("tauri-build");
}
