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

/// Как `encodeURIComponent` в JS.
fn encode_uri_component(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
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
        web_path: format!("/g/p/-/jobs/{}", encode_uri_component(name)),
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

fn ran(start: i64, end: i64, deps: &[&'static str]) -> JobOpts {
    JobOpts {
        start: Some(start),
        end: Some(end),
        deps: deps.to_vec(),
        ..Default::default()
    }
}

fn failed(start: i64, end: i64, deps: &[&'static str]) -> JobOpts {
    JobOpts {
        retried: true,
        status: "FAILED",
        ..ran(start, end, deps)
    }
}

pub fn pipeline_opts(i: usize) -> PipelineOpts {
    PipelineOpts {
        id: format!("gid://gitlab/Ci::Pipeline/{i}").leak(),
        iid: i.to_string().leak(),
        ..Default::default()
    }
}

/// Пайплайн из `insights.test`: ретраи lint и `e2e: [3]`, узкое место build:server.
pub fn sample_pipeline(i: usize) -> RawPipeline {
    raw_pipeline(
        vec![
            job("build:server", "build", ran(0, 302, &[])),
            job("build:static", "build", ran(10, 242, &[])),
            job("lint", "build", failed(0, 50, &[])),
            job("lint", "build", ran(60, 240, &[])),
            job("e2e: [1]", "test", ran(330, 780, &["build:server"])),
            job("e2e: [3]", "test", failed(331, 590, &["build:server"])),
            job("e2e: [3]", "test", failed(600, 850, &["build:server"])),
            job("e2e: [3]", "test", ran(860, 1318, &["build:server"])),
            job(
                "report",
                "report",
                ran(1320, 1400, &["e2e: [1]", "e2e: [3]"]),
            ),
        ],
        pipeline_opts(i),
    )
}

/// Тот же пайплайн без ретраев.
pub fn clean_pipeline(i: usize) -> RawPipeline {
    raw_pipeline(
        vec![
            job("build:server", "build", ran(0, 302, &[])),
            job("build:static", "build", ran(10, 242, &[])),
            job("lint", "build", ran(60, 240, &[])),
            job("e2e: [1]", "test", ran(330, 780, &["build:server"])),
            job("e2e: [3]", "test", ran(860, 1318, &["build:server"])),
            job(
                "report",
                "report",
                ran(1320, 1400, &["e2e: [1]", "e2e: [3]"]),
            ),
        ],
        pipeline_opts(i),
    )
}
