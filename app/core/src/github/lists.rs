//! Списки GitHub для формы и агрегата (спека, § 6–7).

use std::collections::BTreeMap;

use serde::Deserialize;
use time::OffsetDateTime;

use super::{Client, WireRun, status};
use crate::browse::{Commit, Page, Pipeline, Project, Workflow};
use crate::error::{Error, ErrorCode};
use crate::gitlab::{Listed, PipelineFilter};
use crate::iso::iso;

const MAX_LIST_PAGES: usize = 10;
const RECENT_PER_PAGE: usize = 20;
const MAX_BRANCHES: usize = 20;

#[derive(Deserialize)]
struct Runs {
    workflow_runs: Vec<WireRun>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    pushed_at: Option<String>,
    default_branch: Option<String>,
}

impl From<Repo> for Project {
    fn from(r: Repo) -> Self {
        Project {
            full_path: r.full_name.clone(),
            name: r.full_name,
            last_activity_at: r.pushed_at,
            default_branch: r.default_branch,
        }
    }
}

#[derive(Deserialize)]
struct Branch {
    name: String,
}

#[derive(Deserialize)]
struct Pull {
    head: Head,
}

#[derive(Deserialize)]
struct Head {
    sha: String,
}

#[derive(Deserialize)]
struct Workflows {
    workflows: Vec<WireWorkflow>,
}

#[derive(Deserialize)]
struct WireWorkflow {
    name: String,
    path: String,
    state: String,
}

/// Как `encodeURIComponent`: ветка `feature/a+b` не должна сломать запрос.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `?a=1&b=2` без пустых параметров.
fn query(params: &[(&str, Option<String>)]) -> String {
    let parts: Vec<String> = params
        .iter()
        .filter_map(|(key, value)| value.as_deref().map(|v| format!("{key}={}", encode(v))))
        .collect();
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

fn runs_path(project: &str, workflow: Option<&str>) -> String {
    match workflow {
        Some(file) => format!("/repos/{project}/actions/workflows/{}/runs", encode(file)),
        None => format!("/repos/{project}/actions/runs"),
    }
}

/// Статус формы в параметр `status` — только когда он совпадает со статусом таблицы 2.3 точно (спека, § 7).
fn status_param(status: &str) -> Option<&'static str> {
    match status {
        "SUCCESS" => Some("success"),
        "CANCELED" => Some("cancelled"),
        _ => None,
    }
}

/// Курсор страниц GitHub — номер страницы текстом.
fn page_number(after: Option<&str>) -> usize {
    after.and_then(|a| a.parse().ok()).unwrap_or(1)
}

fn to_pipeline(run: WireRun) -> Pipeline {
    Pipeline {
        id: run.id.to_string(),
        iid: run.run_number.to_string(),
        status: status(&run.status, run.conclusion.as_deref()).to_lowercase(),
        source: Some(run.event.clone()),
        created_at: iso(run.created_at),
        duration: run
            .completed()
            .then(|| run.duration(run.updated_at).whole_milliseconds() as i64),
        commit: run.head_commit.as_ref().map(|c| Commit {
            sha: run.head_sha.chars().take(8).collect(),
            title: c.message.lines().next().unwrap_or_default().into(),
        }),
        author: run.actor.as_ref().map(|a| a.login.clone()),
        url: run.html_url.clone(),
    }
}

impl Client {
    fn project_not_found(&self, project: &str) -> Error {
        Error::new(ErrorCode::ProjectNotFound)
            .with("host", self.host())
            .with("project", project)
    }

    pub(crate) async fn repo(&self, project: &str) -> Result<Project, Error> {
        let repo: Repo = self
            .json(&format!("/repos/{project}"))
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(repo.into())
    }

    /// Свои репозитории от недавно изменённых; поиск — по загруженной странице (у REST его нет).
    pub(crate) async fn repos(
        &self,
        search: &str,
        after: Option<&str>,
    ) -> Result<Page<Project>, Error> {
        let page = page_number(after);
        let (repos, next): (Vec<Repo>, _) = self
            .page(&format!("/user/repos?sort=pushed&per_page=100&page={page}"))
            .await?
            .unwrap_or_default();
        let needle = search.trim().to_lowercase();
        Ok(Page {
            items: repos
                .into_iter()
                .filter(|r| r.full_name.to_lowercase().contains(&needle))
                .map(Project::from)
                .collect(),
            next: next.map(|_| (page + 1).to_string()),
        })
    }

    /// Ветки первой страницы по подстроке; основная — первой.
    pub(crate) async fn branches(&self, project: &str, search: &str) -> Result<Vec<String>, Error> {
        let root = self.repo(project).await?.default_branch;
        let found: Vec<Branch> = self
            .json(&format!("/repos/{project}/branches?per_page=100"))
            .await?
            .unwrap_or_default();
        let needle = search.trim().to_lowercase();
        let mut names: Vec<String> = found
            .into_iter()
            .map(|b| b.name)
            .filter(|n| n.to_lowercase().contains(&needle))
            .collect();
        if let Some(i) = root.and_then(|root| names.iter().position(|n| *n == root)) {
            names[..=i].rotate_right(1);
        }
        names.truncate(MAX_BRANCHES);
        Ok(names)
    }

    pub(crate) async fn recent(
        &self,
        project: &str,
        r#ref: Option<&str>,
        workflow: Option<&str>,
        after: Option<&str>,
    ) -> Result<Page<Pipeline>, Error> {
        let page = page_number(after);
        let target = format!(
            "{}{}",
            runs_path(project, workflow),
            query(&[
                ("branch", r#ref.map(String::from)),
                ("per_page", Some(RECENT_PER_PAGE.to_string())),
                ("page", Some(page.to_string())),
            ])
        );
        let (runs, next): (Runs, _) = self
            .page(&target)
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(Page {
            items: runs.workflow_runs.into_iter().map(to_pipeline).collect(),
            next: next.map(|_| (page + 1).to_string()),
        })
    }

    pub(crate) async fn workflow_list(&self, project: &str) -> Result<Vec<Workflow>, Error> {
        let found: Workflows = self
            .json(&format!("/repos/{project}/actions/workflows?per_page=100"))
            .await?
            .ok_or_else(|| self.project_not_found(project))?;
        Ok(found
            .workflows
            .into_iter()
            .filter(|w| w.state == "active")
            .filter_map(|w| {
                // динамические workflow (Dependabot, Copilot) файла не имеют
                let file = w.path.strip_prefix(".github/workflows/")?;
                (file.ends_with(".yml") || file.ends_with(".yaml")).then(|| Workflow {
                    file: file.to_string(),
                    name: w.name,
                })
            })
            .collect())
    }

    /// Запуски по фильтру агрегата, не больше `filter.last`; несколько статусов — фильтр на клиенте.
    pub(crate) async fn list_runs(
        &self,
        project: &str,
        filter: &PipelineFilter,
    ) -> Result<Listed, Error> {
        let single = filter
            .statuses
            .as_ref()
            .filter(|s| s.len() == 1)
            .and_then(|s| status_param(&s[0]));
        let mut next = Some(format!(
            "{}{}",
            runs_path(project, filter.workflow.as_deref()),
            query(&[
                ("branch", filter.r#ref.clone()),
                ("event", filter.source.clone()),
                ("status", single.map(String::from)),
                ("per_page", Some("100".into())),
            ])
        ));
        let mut listed = Listed {
            ids: Vec::new(),
            counts: BTreeMap::new(),
        };
        for _ in 0..MAX_LIST_PAGES {
            if listed.ids.len() >= filter.last {
                break;
            }
            let Some(target) = next.take() else { break };
            let (runs, link): (Runs, _) = self
                .page(&target)
                .await?
                .ok_or_else(|| self.project_not_found(project))?;
            for run in runs.workflow_runs {
                if listed.ids.len() >= filter.last {
                    break;
                }
                let s = status(&run.status, run.conclusion.as_deref());
                if filter
                    .statuses
                    .as_ref()
                    .is_some_and(|w| !w.iter().any(|w| w == s))
                {
                    continue;
                }
                listed.ids.push(run.id.to_string());
                *listed.counts.entry(s.to_string()).or_default() += 1;
            }
            next = link;
        }
        Ok(listed)
    }

    /// Самый долгий run head-коммита PR (спека, § 6).
    pub(crate) async fn head_run(&self, project: &str, number: &str) -> Result<String, Error> {
        let none = || {
            Error::new(ErrorCode::MrHasNoPipeline)
                .with("iid", number)
                .with("project", project)
        };
        let pull: Pull = self
            .json(&format!("/repos/{project}/pulls/{number}"))
            .await?
            .ok_or_else(none)?;
        let runs: Runs = self
            .json(&format!(
                "/repos/{project}/actions/runs?head_sha={}&per_page=100",
                pull.head.sha
            ))
            .await?
            .ok_or_else(none)?;
        let now = OffsetDateTime::now_utc();
        runs.workflow_runs
            .iter()
            .max_by_key(|r| r.duration(now))
            .map(|r| r.id.to_string())
            .ok_or_else(none)
    }
}
