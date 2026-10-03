use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use time::OffsetDateTime;

static SHARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\s+[0-9]+/[0-9]+|:\s*\[[^\]]*\])$").expect("static regex"));

/// Имя без суффикса шарда: `rspec 3/10` и `e2e: [1]` → `rspec`, `e2e`.
pub fn shard_group_name(name: &str) -> &str {
    SHARD_RE.find(name).map_or(name, |m| &name[..m.start()])
}

/// Миллисекунды от создания корневого пайплайна.
pub type Ms = i64;

/// Пайплайн, как его отдаёт GitLab: джобы — от новых к старым.
#[derive(Debug, Clone)]
pub struct RawPipeline {
    pub project: String,
    pub pipeline: RawPipelineInfo,
    pub jobs: Vec<RawJob>,
    /// downstream-пайплайны по id bridge-джобы
    pub downstream: HashMap<String, RawPipeline>,
}

#[derive(Debug, Clone)]
pub struct RawPipelineInfo {
    pub id: String,
    pub iid: String,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub finished_at: Option<OffsetDateTime>,
    pub r#ref: String,
    pub path: String,
    /// объявленный порядок стейджей; пусто — порядок появления джоб
    pub stages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RawJob {
    pub id: String,
    pub name: String,
    pub bridge: bool,
    pub status: String,
    pub started_at: Option<OffsetDateTime>,
    pub finished_at: Option<OffsetDateTime>,
    /// секунды
    pub queued_duration: Option<f64>,
    pub retried: bool,
    pub allow_failure: bool,
    pub web_path: String,
    pub stage: String,
    /// имена джоб из `previousStageJobsOrNeeds`
    pub needs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Pipeline,
    Stage,
    Group,
    Job,
    Bridge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    pub start: Option<Ms>,
    pub end: Option<Ms>,
    pub status: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    /// джоба и bridge
    pub stage: Option<String>,
    /// стейдж
    pub position: Option<usize>,
    /// пайплайн
    pub project: Option<String>,
    pub r#ref: Option<String>,
    pub start: Option<Ms>,
    pub end: Option<Ms>,
    pub queued: Option<Ms>,
    pub status: Option<String>,
    pub url: Option<String>,
    pub allow_failure: bool,
    pub attempts: Vec<Attempt>,
    pub deps: Vec<String>,
    pub children: Vec<Span>,
    /// сосед, после которого узел стоит в родителе
    pub after: Option<String>,
}

fn span(id: String, kind: Kind, name: String) -> Span {
    Span {
        id,
        kind,
        name,
        stage: None,
        position: None,
        project: None,
        r#ref: None,
        start: None,
        end: None,
        queued: None,
        status: None,
        url: None,
        allow_failure: false,
        attempts: vec![],
        deps: vec![],
        children: vec![],
        after: None,
    }
}

/// Без старта — в конец.
fn by_start(s: &Span) -> (bool, Option<Ms>) {
    (s.start.is_none(), s.start)
}

fn bounds(spans: &[Span]) -> (Option<Ms>, Option<Ms>) {
    let start = spans.iter().filter_map(|s| s.start).min();
    let end = spans.iter().filter_map(|s| s.end).max();
    (start, end)
}

fn leaf_ids(s: &Span) -> Vec<&str> {
    match s.kind {
        Kind::Group => s.children.iter().flat_map(leaf_ids).collect(),
        Kind::Pipeline | Kind::Stage | Kind::Job | Kind::Bridge => vec![&s.id],
    }
}

/// Порядок соседей: узел встаёт сразу за соседом, которого ждал дольше всех
/// (он же — `after`), остальные — по старту.
fn order_by_deps(mut children: Vec<Span>) -> Vec<Span> {
    children.sort_by_key(by_start);
    // зависимость может указывать на шард: владелец шарда — его группа среди соседей
    let mut owner: HashMap<&str, usize> = HashMap::new();
    for (i, s) in children.iter().enumerate() {
        owner.insert(&s.id, i);
        for id in leaf_ids(s) {
            owner.insert(id, i);
        }
    }

    // None < Some: конец без значения проигрывает любому
    let parent_of: Vec<Option<usize>> = children
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut parent: Option<usize> = None;
            for &o in s.deps.iter().filter_map(|id| owner.get(id.as_str())) {
                let better = o != i
                    && parent.is_none_or(|p| {
                        let (oe, pe) = (children[o].end, children[p].end);
                        oe > pe || (oe == pe && o < p)
                    });
                if better {
                    parent = Some(o);
                }
            }
            parent
        })
        .collect();

    let mut kids = vec![vec![]; children.len()];
    let mut roots = vec![];
    for (i, p) in parent_of.iter().enumerate() {
        match p {
            Some(p) => kids[*p].push(i),
            None => roots.push(i),
        }
    }

    fn visit(i: usize, kids: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[i] {
            return;
        }
        seen[i] = true;
        order.push(i);
        for &k in &kids[i] {
            visit(k, kids, seen, order);
        }
    }
    let mut seen = vec![false; children.len()];
    let mut order = vec![];
    // узлы из цикла зависимостей не достижимы от корней — добираем их по старту
    for i in roots.into_iter().chain(0..children.len()) {
        visit(i, &kids, &mut seen, &mut order);
    }

    let after: Vec<Option<String>> = parent_of
        .iter()
        .map(|p| p.map(|p| children[p].id.clone()))
        .collect();
    let mut slots: Vec<Option<Span>> = children.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|i| Span {
            after: after[i].clone(),
            ..slots[i].take().unwrap()
        })
        .collect()
}

/// Дерево пайплайна: время в мс от создания `raw`.
pub fn build_tree(raw: &RawPipeline, base_url: &str, now: OffsetDateTime) -> Span {
    build(raw, base_url, raw.pipeline.created_at, now)
}

fn build(raw: &RawPipeline, base_url: &str, origin: OffsetDateTime, now: OffsetDateTime) -> Span {
    let at = |t: OffsetDateTime| (t - origin).whole_milliseconds() as Ms;
    let end_of = |j: &RawJob| j.started_at.map(|_| at(j.finished_at.unwrap_or(now)));
    let ordered: Vec<&RawJob> = raw.jobs.iter().rev().collect();
    let current: Vec<&RawJob> = ordered.iter().copied().filter(|j| !j.retried).collect();
    let id_by_name: HashMap<&str, &str> = current
        .iter()
        .map(|j| (j.name.as_str(), j.id.as_str()))
        .collect();

    let mut jobs: Vec<Span> = current
        .iter()
        .map(|j| {
            let children: Vec<Span> = raw
                .downstream
                .get(&j.id)
                .map(|ds| build(ds, base_url, origin, now))
                .into_iter()
                .collect();
            // bridge заканчивается не раньше своего downstream
            let end = match (end_of(j), children.first().and_then(|ds| ds.end)) {
                (own, None) => own,
                (own, Some(ds)) => Some(own.unwrap_or(ds).max(ds)),
            };
            Span {
                stage: Some(j.stage.clone()),
                start: j.started_at.map(at),
                end,
                children,
                queued: j.queued_duration.map(|q| (q * 1000.0).round() as Ms),
                status: Some(j.status.to_lowercase()),
                allow_failure: j.allow_failure,
                url: Some(format!("{base_url}{}", j.web_path)),
                attempts: ordered
                    .iter()
                    .filter(|r| r.retried && r.name == j.name)
                    .map(|r| Attempt {
                        start: r.started_at.map(at),
                        end: end_of(r),
                        status: r.status.to_lowercase(),
                        url: format!("{base_url}{}", r.web_path),
                    })
                    .collect(),
                deps: j
                    .needs
                    .iter()
                    .filter_map(|n| id_by_name.get(n.as_str()).map(|&id| id.to_owned()))
                    .collect(),
                ..span(
                    j.id.clone(),
                    if j.bridge { Kind::Bridge } else { Kind::Job },
                    j.name.clone(),
                )
            }
        })
        .collect();

    let p = &raw.pipeline;
    let mut job_stages: Vec<&str> = vec![];
    for j in &ordered {
        if !job_stages.contains(&j.stage.as_str()) {
            job_stages.push(&j.stage);
        }
    }
    // объявленный порядок — правда GitLab; стейджи без объявления — в порядке появления джоб
    let mut stage_names: Vec<&str> = p
        .stages
        .iter()
        .map(String::as_str)
        .filter(|s| job_stages.contains(s))
        .collect();
    for s in job_stages {
        if !stage_names.contains(&s) {
            stage_names.push(s);
        }
    }
    let stages: Vec<Span> = stage_names
        .iter()
        .enumerate()
        .map(|(position, &stage)| {
            let mut members: Vec<Span> = jobs
                .extract_if(.., |s| s.stage.as_deref() == Some(stage))
                .collect();
            members.sort_by_key(by_start);
            let groups: Vec<String> = members
                .iter()
                .map(|s| shard_group_name(&s.name).to_owned())
                .collect();
            let mut slots: Vec<Option<Span>> = members.into_iter().map(Some).collect();
            let mut children = vec![];
            for i in 0..slots.len() {
                let Some(first) = slots[i].take() else {
                    continue;
                };
                let mut shards = vec![first];
                for k in i + 1..slots.len() {
                    if groups[k] == groups[i] {
                        shards.extend(slots[k].take());
                    }
                }
                if shards.len() < 2 {
                    children.extend(shards);
                    continue;
                }
                let (start, end) = bounds(&shards);
                let mut deps: Vec<String> = vec![];
                for d in shards.iter().flat_map(|s| &s.deps) {
                    if !deps.contains(d) {
                        deps.push(d.clone());
                    }
                }
                children.push(Span {
                    start,
                    end,
                    deps,
                    children: order_by_deps(shards),
                    ..span(
                        format!("{}:{stage}:{}", p.id, groups[i]),
                        Kind::Group,
                        groups[i].clone(),
                    )
                });
            }
            let (start, end) = bounds(&children);
            Span {
                position: Some(position),
                start,
                end,
                children: order_by_deps(children),
                ..span(format!("{}:{stage}", p.id), Kind::Stage, stage.to_owned())
            }
        })
        .collect();

    let stages_end = bounds(&stages).1;
    let finished = p.finished_at.map(at);
    Span {
        project: Some(raw.project.clone()),
        r#ref: Some(p.r#ref.clone()),
        start: Some(at(p.created_at)),
        end: match finished {
            None => stages_end,
            Some(f) => Some(f.max(stages_end.unwrap_or(f))),
        },
        status: Some(p.status.to_lowercase()),
        url: Some(format!("{base_url}{}", p.path)),
        children: stages,
        ..span(p.id.clone(), Kind::Pipeline, format!("#{}", p.iid))
    }
}
