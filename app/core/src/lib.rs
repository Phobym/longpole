//! Ядро Longpole: без зависимостей от Tauri.

pub mod aggregate;
pub mod browse;
pub mod critical_path;
pub mod error;
pub mod github;
pub mod gitlab;
#[cfg(feature = "desktop")]
pub mod history;
pub mod hosts;
pub mod insights;
mod iso;
#[cfg(feature = "desktop")]
mod json_file;
pub mod model;
#[cfg(feature = "desktop")]
pub mod oauth;
#[cfg(feature = "desktop")]
pub mod projects;
pub mod render;
pub mod report;
pub mod request;
pub mod schema;
#[cfg(feature = "desktop")]
pub mod settings;
pub mod source;
#[cfg(feature = "desktop")]
pub mod tokens;
pub mod web_oauth;
