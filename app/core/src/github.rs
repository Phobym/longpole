//! GitHub Actions: workflow run как пайплайн (спека, § 2), списки для формы (§ 6).

mod client;
pub(crate) mod workflow;

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::{Arc, PoisonError};

use serde::Deserialize;
use time::OffsetDateTime;

pub use client::{Client, probe, probe_at};

use crate::error::{Error, ErrorCode};
use crate::model::{RawJob, RawPipeline, RawPipelineInfo};
use workflow::{Matched, Workflow};

const MAX_JOB_PAGES: usize = 50;
const JOBS_PER_PAGE: usize = 100;

#[derive(Deserialize)]
pub(crate) struct WireRun {
    pub(crate) id: u64,
    name: Option<String>,
    pub(crate) run_number: u64,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) updated_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub(crate) run_started_at: Option<OffsetDateTime>,
    head_branch: Option<String>,
    pub(crate) head_sha: String,
    path: String,
    pub(crate) html_url: String,
    pub(crate) event: String,
    pub(crate) head_commit: Option<HeadCommit>,
    pub(crate) actor: Option<Actor>,
}

#[derive(Deserialize)]
pub(crate) struct HeadCommit {
    pub(crate) message: String,
}

#[derive(Deserialize)]
pub(crate) struct Actor {
    pub(crate) login: String,
}

impl WireRun {
    pub(crate) fn completed(&self) -> bool {
        self.status == "completed"
    }

    /// От старта последней попытки до конца; идущий — до `now`.
    pub(crate) fn duration(&self, now: OffsetDateTime) -> time::Duration {
        let end = if self.completed() {
            self.updated_at
        } else {
            now
        };
        end - self.run_started_at.unwrap_or(self.created_at)
    }
}

#[derive(Deserialize)]
struct WireJob {
    id: u64,
    run_attempt: u32,
    name: String,
    status: String,
    conclusion: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    completed_at: Option<OffsetDateTime>,
    html_url: Option<String>,
}

#[derive(Deserialize)]
struct Jobs {
    jobs: Vec<WireJob>,
}

/// Статус GitHub в словаре GitLab (спека, § 2.3); неизвестный итог завершённой — `FAILED`.
pub(crate) fn status(status: &str, conclusion: Option<&str>) -> &'static str {
    match (status, conclusion) {
        ("completed", Some("success")) => "SUCCESS",
        ("completed", Some("cancelled")) => "CANCELED",
        ("completed", Some("skipped" | "neutral")) => "SKIPPED",
        ("completed", Some("action_required")) => "MANUAL",
        ("completed", _) => "FAILED",
        ("waiting" | "pending" | "requested", _) => "MANUAL",
        ("queued", _) => "PENDING",
        _ => "RUNNING",
    }
}

/// `https://host/o/r/…` → `/o/r/…`; не адрес — пусто.
pub(crate) fn path_of(url: &str) -> String {
    url.split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|i| rest[i..].to_string()))
        .unwrap_or_default()
}

/// Пропущенной джобе GitHub ставит старт позже конца: времён у неё нет.
fn without_skipped_times(mut job: WireJob) -> WireJob {
    if job.conclusion.as_deref() == Some("skipped") {
        job.started_at = None;
        job.completed_at = None;
    }
    job
}

/// Джобы всех попыток (спека, § 2.6): последняя попытка — основная; ранняя с теми же временами —
/// копия, не запускавшаяся — не ретрай; остальные ранние — `retried`.
fn attempts(mut jobs: Vec<WireJob>) -> Vec<(WireJob, bool)> {
    jobs.sort_by_key(|j| Reverse(j.run_attempt));
    let mut seen: HashMap<String, Vec<(Option<OffsetDateTime>, Option<OffsetDateTime>)>> =
        HashMap::new();
    let mut kept = Vec::new();
    for job in jobs {
        let times = (job.started_at, job.completed_at);
        let earlier = seen.entry(job.name.clone()).or_default();
        let retried = !earlier.is_empty();
        if earlier.contains(&times) || (retried && times.0.is_none()) {
            continue;
        }
        earlier.push(times);
        kept.push((job, retried));
    }
    kept
}

type Found<'a> = (WireJob, bool, Option<Matched<'a>>);

/// Ключ YAML → имена джоб API последней попытки: из них `needs`.
fn names_by_key(found: &[Found<'_>]) -> HashMap<String, Vec<String>> {
    let mut names: HashMap<String, Vec<String>> = HashMap::new();
    for (_, retried, matched) in found {
        if let Some(m) = matched
            && !retried
        {
            names
                .entry(m.job.key.clone())
                .or_default()
                .push(m.name.clone());
        }
    }
    names
}

fn raw_job(
    job: &WireJob,
    retried: bool,
    matched: Option<&Matched<'_>>,
    names: &HashMap<String, Vec<String>>,
    stages: &[String],
) -> RawJob {
    let needs = matched
        .map(|m| {
            m.job
                .needs
                .iter()
                .flat_map(|key| names.get(key).into_iter().flatten().cloned())
                .collect()
        })
        .unwrap_or_default();
    RawJob {
        id: job.id.to_string(),
        name: matched.map_or_else(|| job.name.clone(), |m| m.name.clone()),
        bridge: false,
        status: status(&job.status, job.conclusion.as_deref()).into(),
        started_at: job.started_at,
        finished_at: job.completed_at,
        // у копии из ранней попытки `created_at` позже `started_at`: очереди не было
        queued_duration: job
            .started_at
            .map(|s| (s - job.created_at).as_seconds_f64())
            .filter(|q| *q >= 0.0),
        retried,
        allow_failure: matched.is_some_and(|m| m.job.allow_failure),
        web_path: job.html_url.as_deref().map(path_of).unwrap_or_default(),
        stage: stages[matched.map_or(0, |m| m.job.level)].clone(),
        needs,
    }
}

/// Run и его джобы → `RawPipeline`; `workflow == None` — файл не загрузился.
fn to_raw(
    project: &str,
    run: &WireRun,
    jobs: Vec<WireJob>,
    workflow: Option<&Workflow>,
) -> RawPipeline {
    let mut found: Vec<Found<'_>> = attempts(jobs.into_iter().map(without_skipped_times).collect())
        .into_iter()
        .map(|(job, retried)| {
            let matched = workflow.and_then(|w| w.find(&job.name));
            (job, retried, matched)
        })
        .collect();
    // как у GitLab: от новых к старым
    found.sort_by_key(|(job, ..)| Reverse(job.created_at));
    let names = names_by_key(&found);
    let stages = match workflow {
        Some(w) => w.stage_names(),
        None => vec![run.name.clone().unwrap_or_default()],
    };
    let jobs = found
        .iter()
        .map(|(job, retried, matched)| raw_job(job, *retried, matched.as_ref(), &names, &stages))
        .collect();

    RawPipeline {
        project: project.into(),
        pipeline: RawPipelineInfo {
            id: run.id.to_string(),
            iid: run.run_number.to_string(),
            status: status(&run.status, run.conclusion.as_deref()).into(),
            created_at: run.created_at,
            finished_at: run.completed().then_some(run.updated_at),
            r#ref: run.head_branch.clone().unwrap_or_default(),
            path: path_of(&run.html_url),
            stages,
        },
        jobs,
        downstream: HashMap::new(),
        needs_missing: workflow.is_none(),
    }
}

impl Client {
    /// Workflow run со всеми попытками (спека, § 2).
    pub async fn fetch_run(&self, project: &str, id: &str) -> Result<RawPipeline, Error> {
        let not_found = || {
            Error::new(ErrorCode::PipelineNotFound)
                .with("host", self.host())
                .with("pipeline", id)
                .with("project", project)
        };
        let run: WireRun = self
            .json(&format!("/repos/{project}/actions/runs/{id}"))
            .await?
            .ok_or_else(not_found)?;
        let mut jobs = Vec::new();
        let mut next = Some(format!(
            "/repos/{project}/actions/runs/{id}/jobs?filter=all&per_page={JOBS_PER_PAGE}"
        ));
        for _ in 0..MAX_JOB_PAGES {
            let Some(target) = next.take() else { break };
            let (page, link): (Jobs, _) = self.page(&target).await?.ok_or_else(not_found)?;
            jobs.extend(page.jobs);
            next = link;
        }
        if next.is_some() {
            return Err(Error::new(ErrorCode::TooManyJobs)
                .with("pipeline", id)
                .with("project", project)
                .with("limit", MAX_JOB_PAGES * JOBS_PER_PAGE));
        }
        let workflow = self.workflow(project, &run).await;
        Ok(to_raw(project, &run, jobs, workflow.as_deref()))
    }

    /// Файл workflow на коммите запуска; любой сбой — `None`: отчёт строится без `needs` (спека, § 2.4); в кэше только окончательные исходы.
    // ponytail: параллельные запуски одного агрегата могут скачать файл одновременно — лишний запрос, не ошибка
    async fn workflow(&self, project: &str, run: &WireRun) -> Option<Arc<Workflow>> {
        let path = run.path.split('@').next().unwrap_or(&run.path);
        let key = (path.to_string(), run.head_sha.clone());
        let cached = self
            .workflows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .cloned();
        if let Some(cached) = cached {
            return cached;
        }
        // временная ошибка (лимит, сеть) в кэш не идёт: следующий запуск попробует снова
        let text = self
            .raw(&format!(
                "/repos/{project}/contents/{path}?ref={}",
                run.head_sha
            ))
            .await
            .ok()?;
        let parsed = text.as_deref().and_then(workflow::parse).map(Arc::new);
        self.workflows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, parsed.clone());
        parsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn статусы_github_в_словаре_gitlab() {
        let cases = [
            ("completed", Some("success"), "SUCCESS"),
            ("completed", Some("failure"), "FAILED"),
            ("completed", Some("timed_out"), "FAILED"),
            ("completed", Some("startup_failure"), "FAILED"),
            ("completed", Some("cancelled"), "CANCELED"),
            ("completed", Some("skipped"), "SKIPPED"),
            ("completed", Some("neutral"), "SKIPPED"),
            ("completed", Some("action_required"), "MANUAL"),
            ("waiting", None, "MANUAL"),
            ("pending", None, "MANUAL"),
            ("requested", None, "MANUAL"),
            ("queued", None, "PENDING"),
            ("in_progress", None, "RUNNING"),
        ];
        for (s, c, expected) in cases {
            assert_eq!(status(s, c), expected, "{s} {c:?}");
        }
    }

    #[test]
    fn путь_из_адреса() {
        assert_eq!(
            path_of("https://github.com/o/r/actions/runs/7"),
            "/o/r/actions/runs/7"
        );
        assert_eq!(path_of("не адрес"), "");
    }
}
