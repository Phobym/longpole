//! Ядро pipeline-trace: без зависимостей от Tauri.

pub mod aggregate;
pub mod browse;
pub mod critical_path;
pub mod error;
pub mod gitlab;
pub mod history;
pub mod hosts;
pub mod insights;
mod iso;
mod json_file;
pub mod model;
pub mod projects;
pub mod render;
pub mod report;
pub mod request;
pub mod schema;
pub mod settings;
pub mod tokens;
