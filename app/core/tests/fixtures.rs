//! Билдеры сырых пайплайнов по `test/fixtures.mjs`.
#![allow(dead_code)]

use std::collections::HashMap;

use pipeline_trace_core::model::{RawJob, RawPipeline, RawPipelineInfo};
use time::{Duration, OffsetDateTime, macros::datetime};

pub const T0: OffsetDateTime = datetime!(2026-09-24 10:00:00 +03:00);

pub fn at(sec: i64) -> OffsetDateTime {
    T0 + Duration::seconds(sec)
}

pub struct JobOpts {
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub deps: Vec<&'static str>,
    pub retried: bool,
    pub status: &'static str,
    pub bridge: bool,
    pub queued: f64,
    pub allow_failure: bool,
    pub id: Option<String>,
}

impl Default for JobOpts {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
            deps: vec![],
            retried: false,
            status: "SUCCESS",
            bridge: false,
            queued: 1.0,
            allow_failure: false,
            id: None,
        }
    }
}

pub fn job(name: &str, stage: &str, o: JobOpts) -> RawJob {
    let retry = if o.retried {
        format!("-r{}", o.start.unwrap_or_default())
    } else {
        String::new()
    };
    RawJob {
        id: o
            .id
            .unwrap_or_else(|| format!("gid://gitlab/Ci::Build/{name}{retry}")),
        name: name.into(),
        bridge: o.bridge,
        status: o.status.into(),
        started_at: o.start.map(at),
        finished_at: o.end.map(at),
        queued_duration: o.start.map(|_| o.queued),
        retried: o.retried,
        allow_failure: o.allow_failure,
        web_path: format!("/g/p/-/jobs/{name}"),
        stage: stage.into(),
        needs: o.deps.iter().map(|&d| d.into()).collect(),
    }
}

pub struct PipelineOpts {
    pub id: &'static str,
    pub iid: &'static str,
    pub status: &'static str,
    pub created_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
    pub project: &'static str,
    pub downstream: HashMap<String, RawPipeline>,
    pub stages: Vec<&'static str>,
}

impl Default for PipelineOpts {
    fn default() -> Self {
        Self {
            id: "gid://gitlab/Ci::Pipeline/1",
            iid: "1",
            status: "SUCCESS",
            created_at: T0,
            finished_at: None,
            project: "g/p",
            downstream: HashMap::new(),
            stages: vec![],
        }
    }
}

/// `jobs` перечисляются в хронологическом порядке, API отдаёт их от новых к старым.
pub fn raw_pipeline(mut jobs: Vec<RawJob>, o: PipelineOpts) -> RawPipeline {
    jobs.reverse();
    RawPipeline {
        project: o.project.into(),
        pipeline: RawPipelineInfo {
            id: o.id.into(),
            iid: o.iid.into(),
            status: o.status.into(),
            created_at: o.created_at,
            finished_at: o.finished_at,
            r#ref: "master".into(),
            path: format!("/{}/-/pipelines/{}", o.project, o.iid),
            stages: o.stages.iter().map(|&s| s.into()).collect(),
        },
        jobs,
        downstream: o.downstream,
    }
}
