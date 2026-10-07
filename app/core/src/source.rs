//! Источник пайплайнов — GitLab или GitHub (спека, § 6). Отчёт и команды формы работают через
//! `Source` и о провайдере не знают.

use std::future::Future;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::browse::{self, Page, Pipeline, Project, Workflow};
use crate::error::Error;
use crate::github;
use crate::gitlab::{self, Gql, Listed, PipelineFilter};
use crate::model::RawPipeline;
use crate::settings::Settings;

/// Хост GitHub, тип которого известен без пробы и кэша.
pub const GITHUB_COM: &str = "github.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Provider {
    Gitlab,
    Github,
}

/// Тип хоста (спека, § 3): кэш, иначе проба `/api/v3/meta` с записью в кэш. Сбой сети или 5xx/429/407 — ошибка, кэш не пишется.
pub async fn resolve_provider(host: &str, settings: &Settings) -> Result<Provider, Error> {
    resolve_provider_at(host, settings, &format!("https://{host}")).await
}

/// `base_url` — адрес хоста; тесты подставляют wiremock.
pub async fn resolve_provider_at(
    host: &str,
    settings: &Settings,
    base_url: &str,
) -> Result<Provider, Error> {
    if let Some(known) = settings.host_kind(host)? {
        return Ok(known);
    }
    let probed = github::probe_at(host, base_url).await?;
    settings.set_host_kind(host, probed)?;
    Ok(probed)
}

/// Операции, которые нужны отчёту и экранам формы.
pub trait Source: Sync {
    fn host(&self) -> &str;

    fn provider(&self) -> Provider;

    /// `id` — то, что отдали `list_pipelines` и `head_pipeline`, или номер из ссылки.
    fn fetch_pipeline(
        &self,
        project: &str,
        id: &str,
    ) -> impl Future<Output = Result<RawPipeline, Error>> + Send;

    fn list_pipelines(
        &self,
        project: &str,
        filter: &PipelineFilter,
    ) -> impl Future<Output = Result<Listed, Error>> + Send;

    /// Пайплайн MR (GitLab) или самый долгий run head-коммита PR (GitHub).
    fn head_pipeline(
        &self,
        project: &str,
        number: &str,
    ) -> impl Future<Output = Result<String, Error>> + Send;

    fn fetch_project(&self, path: &str) -> impl Future<Output = Result<Project, Error>> + Send;

    fn list_projects(
        &self,
        search: &str,
        after: Option<&str>,
    ) -> impl Future<Output = Result<Page<Project>, Error>> + Send;

    fn list_branches(
        &self,
        project: &str,
        search: &str,
    ) -> impl Future<Output = Result<Vec<String>, Error>> + Send;

    fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> impl Future<Output = Result<Page<Pipeline>, Error>> + Send;

    /// У GitLab workflow нет — пустой список.
    fn list_workflows(
        &self,
        project: &str,
    ) -> impl Future<Output = Result<Vec<Workflow>, Error>> + Send;
}

/// GitLab — любой `Gql`: и HTTP-клиент, и фейк тестов.
impl<G: Gql> Source for G {
    fn host(&self) -> &str {
        Gql::host(self)
    }

    fn provider(&self) -> Provider {
        Provider::Gitlab
    }

    async fn fetch_pipeline(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        // номер из ссылки — в gid; список и MR уже отдают gid
        let gid = if id.starts_with("gid://") {
            id.to_string()
        } else {
            format!("gid://gitlab/Ci::Pipeline/{id}")
        };
        gitlab::fetch_pipeline(self, project, &gid).await
    }

    async fn list_pipelines(
        &self,
        project: &str,
        filter: &PipelineFilter,
    ) -> Result<Listed, Error> {
        gitlab::list_pipelines(self, project, filter).await
    }

    async fn head_pipeline(&self, project: &str, number: &str) -> Result<String, Error> {
        gitlab::mr_head_pipeline(self, project, number).await
    }

    async fn fetch_project(&self, path: &str) -> Result<Project, Error> {
        browse::fetch_project(self, path).await
    }

    async fn list_projects(
        &self,
        search: &str,
        after: Option<&str>,
    ) -> Result<Page<Project>, Error> {
        browse::list_projects(self, search, after).await
    }

    async fn list_branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        browse::list_branches(self, project, search).await
    }

    async fn recent_pipelines(
        &self,
        project: &str,
        r#ref: Option<&str>,
        _workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        browse::recent_pipelines(self, project, r#ref, after).await
    }

    async fn list_workflows(&self, _project: &str) -> Result<Vec<Workflow>, Error> {
        Ok(Vec::new())
    }
}
