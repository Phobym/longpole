//! Сборка отчёта: от запроса к GitLab до готового `Report`.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::future::try_join_all;
use regex::Regex;
use serde::Serialize;
use time::OffsetDateTime;
use ts_rs::TS;

use crate::error::{Error, ErrorCode};
use crate::gitlab::PipelineFilter;
use crate::iso::iso;
use crate::model::{Span, build_tree};
use crate::request::Request;
use crate::schema::{Locale, Meta, Report};
use crate::source::Source;

static UNSAFE_IN_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^A-Za-z0-9_.-]+").expect("верный шаблон"));

/// Сколько пайплайнов из скольких загружено.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Progress {
    pub loaded: usize,
    pub total: usize,
}

/// Обстановка сборки: время и язык приходят снаружи, чтобы тесты их фиксировали.
#[derive(Debug, Clone, Copy)]
pub struct BuildEnv {
    pub now: OffsetDateTime,
    pub locale: Locale,
}

#[derive(Debug)]
pub struct Built {
    pub report: Report,
    pub file_name: String,
}

fn slug(text: &str) -> Cow<'_, str> {
    UNSAFE_IN_NAME.replace_all(text, "-")
}

/// `pipeline-trace-<проект>-<суффикс>.html`
pub fn default_file_name(project: &str, suffix: &str) -> String {
    format!("pipeline-trace-{}-{}.html", slug(project), slug(suffix))
}

/// Что загружать: gid пайплайнов, суффикс имени файла и счётчики статусов агрегата.
struct Target {
    ids: Vec<String>,
    suffix: String,
    status_counts: Option<BTreeMap<String, u32>>,
}

async fn resolve(source: &impl Source, request: &Request) -> Result<Target, Error> {
    match request {
        Request::Pipeline { pipeline_id, .. } => Ok(Target {
            ids: vec![pipeline_id.clone()],
            suffix: pipeline_id.clone(),
            status_counts: None,
        }),
        Request::Mr { project, mr_iid } => Ok(Target {
            ids: vec![source.head_pipeline(project, mr_iid).await?],
            suffix: format!("mr{mr_iid}"),
            status_counts: None,
        }),
        Request::Aggregate {
            project,
            r#ref,
            source: event,
            last,
            statuses,
            workflow,
        } => {
            let filter = PipelineFilter {
                r#ref: r#ref.clone(),
                source: event.clone(),
                statuses: statuses
                    .as_ref()
                    .map(|list| list.iter().map(|s| s.as_str().to_string()).collect()),
                last: *last as usize,
                workflow: workflow.clone(),
            };
            let listed = source.list_pipelines(project, &filter).await?;
            if listed.ids.is_empty() {
                return Err(Error::new(ErrorCode::NoPipelines));
            }
            let filter_part = r#ref.as_deref().or(event.as_deref()).unwrap_or("all");
            Ok(Target {
                ids: listed.ids,
                suffix: match workflow {
                    Some(file) => format!("{}-{filter_part}", workflow_stem(file)),
                    None => filter_part.to_string(),
                },
                status_counts: Some(listed.counts),
            })
        }
    }
}

/// `ci.yml` → `ci`.
fn workflow_stem(file: &str) -> &str {
    file.strip_suffix(".yml")
        .or_else(|| file.strip_suffix(".yaml"))
        .unwrap_or(file)
}

async fn load_trees(
    source: &impl Source,
    project: &str,
    ids: &[String],
    env: BuildEnv,
    on_progress: impl Fn(Progress) + Send + Sync,
) -> Result<Vec<Span>, Error> {
    let total = ids.len();
    let loaded = AtomicUsize::new(0);
    on_progress(Progress { loaded: 0, total });
    let raws = try_join_all(ids.iter().map(|id| async {
        let raw = source.fetch_pipeline(project, id).await?;
        let loaded = loaded.fetch_add(1, Ordering::SeqCst) + 1;
        on_progress(Progress { loaded, total });
        Ok::<_, Error>(raw)
    }))
    .await?;
    let base_url = format!("https://{}", source.host());
    Ok(raws
        .iter()
        .map(|raw| build_tree(raw, &base_url, env.now))
        .collect())
}

/// Пайплайн, MR или агрегат по последним пайплайнам; `on_progress` зовут после каждого загруженного пайплайна.
/// Хост берётся у `source`: он один и в запросах, и в ссылках отчёта.
pub async fn build_report(
    source: &impl Source,
    request: &Request,
    env: BuildEnv,
    on_progress: impl Fn(Progress) + Send + Sync,
) -> Result<Built, Error> {
    let target = resolve(source, request).await?;
    let project = request.project();
    let trees = load_trees(source, project, &target.ids, env, on_progress).await?;

    let (label, aggregated) = match request {
        Request::Aggregate {
            r#ref,
            source: event,
            workflow,
            ..
        } => {
            let parts: Vec<&str> = [workflow, r#ref, event]
                .into_iter()
                .filter_map(|p| p.as_deref())
                .collect();
            (Some(parts.join(" · ")).filter(|l| !l.is_empty()), true)
        }
        _ => (Some(trees[0].name.clone()), false),
    };
    let meta = Meta {
        host: source.host().into(),
        project: project.into(),
        label,
        locale: env.locale,
        status_counts: target.status_counts,
        generated_at: iso(env.now),
    };
    let report = if aggregated {
        Report::aggregate(meta, &trees)
    } else {
        Report::single(meta, &trees[0])
    };
    Ok(Built {
        report,
        file_name: default_file_name(project, &target.suffix),
    })
}
