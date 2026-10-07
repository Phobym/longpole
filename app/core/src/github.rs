//! GitHub Actions: workflow run как пайплайн (спека, § 2), списки для формы (§ 6).

mod client;
pub(crate) mod workflow;

pub use client::{Client, probe, probe_at};
