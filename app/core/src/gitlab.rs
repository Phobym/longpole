//! GitLab GraphQL: трейт-шов `Gql`, клиент и загрузка пайплайнов.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;

use futures::future::{BoxFuture, try_join_all};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use time::OffsetDateTime;

use crate::error::{Error, ErrorCode};
use crate::model::{RawJob, RawPipeline, RawPipelineInfo};

mod client;

pub use client::Client;

const MAX_DOWNSTREAM_DEPTH: u8 = 3;
const MAX_LIST_PAGES: usize = 10;
const MAX_JOB_PAGES: usize = 50;
/// `first:` в `PIPELINE_QUERY`; больше GitLab не отдаёт.
const JOBS_PER_PAGE: usize = 100;

/// Один GraphQL-запрос; `Ok` — содержимое `data`.
pub trait Gql: Sync {
    fn host(&self) -> &str;

    /// Ошибка с `host` — общим параметром кодов, которым нужен хост GitLab.
    fn error(&self, code: ErrorCode) -> Error {
        Error::new(code).with("host", self.host())
    }

    fn query(
        &self,
        query: &str,
        variables: Value,
    ) -> impl Future<Output = Result<Value, Error>> + Send;
}

/// Запрос с разбором `data`; чужая форма ответа — `graphql` с текстом serde.
pub(crate) async fn query_as<T: DeserializeOwned>(
    gql: &impl Gql,
    query: &str,
    variables: Value,
) -> Result<T, Error> {
    let data = gql.query(query, variables).await?;
    serde_json::from_value(data).map_err(|e| gql.error(ErrorCode::Graphql).with("detail", e))
}

#[derive(Deserialize)]
struct ProjectData<T> {
    project: Option<T>,
}

/// Запрос с корнем `project`: `null` — проекта нет или нет доступа.
pub(crate) async fn query_project<T: DeserializeOwned>(
    gql: &impl Gql,
    query: &str,
    variables: Value,
    project: &str,
) -> Result<T, Error> {
    let data: ProjectData<T> = query_as(gql, query, variables).await?;
    data.project.ok_or_else(|| {
        gql.error(ErrorCode::ProjectNotFound)
            .with("project", project)
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

impl PageInfo {
    pub(crate) fn next(self) -> Option<String> {
        self.end_cursor.filter(|_| self.has_next_page)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Connection<T> {
    pub(crate) page_info: PageInfo,
    pub(crate) nodes: Vec<T>,
}

#[derive(Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}

#[derive(Deserialize)]
struct Named {
    name: String,
}

const PIPELINE_QUERY: &str = "query($project: ID!, $id: CiPipelineID!, $after: String) {
  project(fullPath: $project) {
    pipeline(id: $id) {
      id iid status createdAt finishedAt ref path
      stages { nodes { name } }
      jobs(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id name kind status startedAt finishedAt queuedDuration retried allowFailure webPath
          stage { name }
          previousStageJobsOrNeeds { nodes { ... on CiBuildNeed { name } ... on CiJob { name } } }
          downstreamPipeline { id project { fullPath } }
        }
      }
    }
  }
}";

#[derive(Deserialize)]
struct PipelineProject {
    pipeline: Option<WirePipeline>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WirePipeline {
    id: String,
    iid: String,
    status: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    finished_at: Option<OffsetDateTime>,
    // в схеме GitLab `ref` и `path` могут быть null
    r#ref: Option<String>,
    path: Option<String>,
    stages: Nodes<Named>,
    jobs: Connection<WireJob>,
}

impl WirePipeline {
    fn split(self) -> (RawPipelineInfo, Connection<WireJob>) {
        let info = RawPipelineInfo {
            id: self.id,
            iid: self.iid,
            status: self.status,
            created_at: self.created_at,
            finished_at: self.finished_at,
            r#ref: self.r#ref.unwrap_or_default(),
            path: self.path.unwrap_or_default(),
            stages: self.stages.nodes.into_iter().map(|s| s.name).collect(),
        };
        (info, self.jobs)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireJob {
    id: String,
    name: String,
    kind: String,
    status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    started_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    finished_at: Option<OffsetDateTime>,
    queued_duration: Option<f64>,
    retried: bool,
    allow_failure: bool,
    web_path: Option<String>,
    stage: Named,
    previous_stage_jobs_or_needs: Nodes<Named>,
    downstream_pipeline: Option<Downstream>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Downstream {
    id: String,
    project: ProjectPath,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectPath {
    full_path: String,
}

impl WireJob {
    fn split(self) -> (RawJob, Option<Downstream>) {
        let job = RawJob {
            id: self.id,
            name: self.name,
            bridge: self.kind == "BRIDGE",
            status: self.status,
            started_at: self.started_at,
            finished_at: self.finished_at,
            queued_duration: self.queued_duration,
            retried: self.retried,
            allow_failure: self.allow_failure,
            web_path: self.web_path.unwrap_or_default(),
            stage: self.stage.name,
            needs: self
                .previous_stage_jobs_or_needs
                .nodes
                .into_iter()
                .map(|n| n.name)
                .collect(),
        };
        (job, self.downstream_pipeline)
    }
}

/// Bridge-джоба и пайплайн, который она запускает.
struct Bridge {
    job_id: String,
    downstream: Downstream,
}

/// Пайплайн со всеми страницами джоб и bridge'ами, у которых есть downstream.
struct Loaded {
    info: RawPipelineInfo,
    jobs: Vec<RawJob>,
    bridges: Vec<Bridge>,
}

async fn load_pages(gql: &impl Gql, project: &str, id: &str) -> Result<Loaded, Error> {
    let mut jobs = Vec::new();
    let mut bridges = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..MAX_JOB_PAGES {
        let variables = json!({ "project": project, "id": id, "after": after });
        let found: PipelineProject = query_project(gql, PIPELINE_QUERY, variables, project).await?;
        let wire = found.pipeline.ok_or_else(|| {
            gql.error(ErrorCode::PipelineNotFound)
                .with("pipeline", id)
                .with("project", project)
        })?;
        let (info, page) = wire.split();
        for wire_job in page.nodes {
            let (job, downstream) = wire_job.split();
            if let Some(downstream) = downstream {
                bridges.push(Bridge {
                    job_id: job.id.clone(),
                    downstream,
                });
            }
            jobs.push(job);
        }
        let Some(cursor) = page.page_info.next() else {
            return Ok(Loaded {
                info,
                jobs,
                bridges,
            });
        };
        after = Some(cursor);
    }
    Err(Error::new(ErrorCode::TooManyJobs)
        .with("pipeline", id)
        .with("project", project)
        .with("limit", MAX_JOB_PAGES * JOBS_PER_PAGE))
}

/// Пайплайн вместе с downstream до глубины `MAX_DOWNSTREAM_DEPTH`.
pub async fn fetch_pipeline(gql: &impl Gql, project: &str, id: &str) -> Result<RawPipeline, Error> {
    fetch_at_depth(gql, project, id, 0).await
}

fn fetch_at_depth<'a, G: Gql>(
    gql: &'a G,
    project: &'a str,
    id: &'a str,
    depth: u8,
) -> BoxFuture<'a, Result<RawPipeline, Error>> {
    Box::pin(async move {
        let Loaded {
            info,
            jobs,
            bridges,
        } = load_pages(gql, project, id).await?;
        let downstream = if depth < MAX_DOWNSTREAM_DEPTH {
            try_join_all(bridges.iter().map(|bridge| async move {
                let ds = &bridge.downstream;
                let raw = fetch_at_depth(gql, &ds.project.full_path, &ds.id, depth + 1).await?;
                Ok::<_, Error>((bridge.job_id.clone(), raw))
            }))
            .await?
            .into_iter()
            .collect()
        } else {
            HashMap::new()
        };
        Ok(RawPipeline {
            project: project.into(),
            pipeline: info,
            jobs,
            downstream,
        })
    })
}

/// Фильтр «последних N пайплайнов» для агрегата.
#[derive(Debug, Clone)]
pub struct PipelineFilter {
    pub r#ref: Option<String>,
    pub source: Option<String>,
    /// `None` — любые статусы
    pub statuses: Option<Vec<String>>,
    pub last: usize,
}

/// Подошедшие пайплайны (от новых к старым) и сколько их в каком статусе.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub ids: Vec<String>,
    pub counts: BTreeMap<String, u32>,
}

const LIST_QUERY: &str = "query($project: ID!, $ref: String, $source: String, $after: String) {
  project(fullPath: $project) {
    pipelines(ref: $ref, source: $source, first: 100, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes { id status }
    }
  }
}";

#[derive(Deserialize)]
struct PipelinesProject {
    pipelines: Connection<ListedNode>,
}

#[derive(Deserialize)]
struct ListedNode {
    id: String,
    status: String,
}

/// Пайплайны по фильтру, не больше `filter.last`. Статусы сравниваются без учёта регистра:
/// GitLab отдаёт `SUCCESS`, а `browse::Pipeline.status` — `success`.
pub async fn list_pipelines(
    gql: &impl Gql,
    project: &str,
    filter: &PipelineFilter,
) -> Result<Listed, Error> {
    let mut listed = Listed {
        ids: Vec::new(),
        counts: BTreeMap::new(),
    };
    let mut after: Option<String> = None;
    for _ in 0..MAX_LIST_PAGES {
        if listed.ids.len() >= filter.last {
            break;
        }
        let variables = json!({
            "project": project, "ref": filter.r#ref, "source": filter.source, "after": after,
        });
        let found: PipelinesProject = query_project(gql, LIST_QUERY, variables, project).await?;
        for node in found.pipelines.nodes {
            if listed.ids.len() >= filter.last {
                break;
            }
            let wanted = filter
                .statuses
                .as_ref()
                .is_none_or(|s| s.iter().any(|w| w.eq_ignore_ascii_case(&node.status)));
            if !wanted {
                continue;
            }
            listed.ids.push(node.id);
            *listed.counts.entry(node.status).or_default() += 1;
        }
        match found.pipelines.page_info.next() {
            Some(cursor) => after = Some(cursor),
            None => break,
        }
    }
    Ok(listed)
}

const MR_QUERY: &str = "query($project: ID!, $iid: String!) {
  project(fullPath: $project) { mergeRequest(iid: $iid) { headPipeline { id } } }
}";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrProject {
    merge_request: Option<MergeRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MergeRequest {
    head_pipeline: Option<HeadPipeline>,
}

#[derive(Deserialize)]
struct HeadPipeline {
    id: String,
}

/// gid головного пайплайна MR.
pub async fn mr_head_pipeline(gql: &impl Gql, project: &str, iid: &str) -> Result<String, Error> {
    let variables = json!({ "project": project, "iid": iid });
    let found: MrProject = query_project(gql, MR_QUERY, variables, project).await?;
    found
        .merge_request
        .and_then(|mr| mr.head_pipeline)
        .map(|p| p.id)
        .ok_or_else(|| {
            Error::new(ErrorCode::MrHasNoPipeline)
                .with("iid", iid)
                .with("project", project)
        })
}
