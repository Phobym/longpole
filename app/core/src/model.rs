use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;
use time::OffsetDateTime;
use ts_rs::TS;

static SHARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\s+[0-9]+/[0-9]+|:\s*\[[^\]]*\])$").expect("static regex"));

/// Имя без суффикса шарда: `rspec 3/10` и `e2e: [1]` → `rspec`, `e2e`.
pub fn shard_group_name(name: &str) -> &str {
    SHARD_RE.find(name).map_or(name, |m| &name[..m.start()])
}

/// Миллисекунды от создания корневого пайплайна.
pub type Ms = i64;

/// Пайплайн, как его отдаёт GitLab: джобы — от новых к старым.
#[derive(Debug, Clone, PartialEq)]
pub struct RawPipeline {
    pub project: String,
    pub pipeline: RawPipelineInfo,
    pub jobs: Vec<RawJob>,
    /// downstream-пайплайны по id bridge-джобы
    pub downstream: HashMap<String, RawPipeline>,
    /// файл workflow GitHub не загрузился: стейджи и `needs` неизвестны
    pub needs_missing: bool,
}

/// Поля самого пайплайна.
#[derive(Debug, Clone, PartialEq)]
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

/// Джоба или bridge, включая ретраенные попытки.
#[derive(Debug, Clone, PartialEq)]
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
    /// ключ группы шардов; `None` — группа выводится из имени (`shard_group_name`)
    pub shard_group: Option<String>,
}

/// Вид узла дерева.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Kind {
    Pipeline,
    Stage,
    Group,
    Job,
    Bridge,
}

/// p50 и p90; `null` — узел не запускался ни разу.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct P {
    pub p50: Option<Ms>,
    pub p90: Option<Ms>,
}

/// Один запуск узла: `tree` — индекс пайплайна в агрегате.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Sample {
    pub tree: usize,
    pub start: Ms,
    pub end: Ms,
    pub retries: u32,
}

/// Статистика узла агрегата по пайплайнам, где он запускался.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Stats {
    pub present: u32,
    pub total: u32,
    pub start: P,
    pub end: P,
    pub duration: P,
    pub queued: P,
    /// среднее число ретраев на запуск
    pub retries: f64,
    /// запусков с ретраями
    pub retried: u32,
    /// средняя потеря на ретраях, мс
    pub retry_loss: f64,
    /// доля пайплайнов, где узел на критическом пути; у стейджа и группы всегда 0
    pub critical: f64,
    pub samples: Vec<Sample>,
}

/// Прошлая попытка ретраенной джобы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Attempt {
    pub start: Option<Ms>,
    pub end: Option<Ms>,
    pub status: String,
    pub url: String,
}

/// Узел дерева: пайплайн, стейдж, группа шардов, джоба или bridge.
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
    /// узел агрегата
    pub stats: Option<Stats>,
    /// ключ группы шардов из `RawJob`; только пока стейдж собирается
    pub shard_group: Option<String>,
}

impl Span {
    /// Узел без времени, статуса и связей.
    pub fn new(id: impl Into<String>, kind: Kind, name: impl Into<String>) -> Span {
        span(id.into(), kind, name.into())
    }

    /// Обход в глубину: узел раньше своих детей.
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a Span)) {
        f(self);
        for c in &self.children {
            c.walk(f);
        }
    }

    /// Узлы поддерева по id.
    pub fn index(&self) -> HashMap<&str, &Span> {
        let mut by_id = HashMap::new();
        self.walk(&mut |s| {
            by_id.insert(s.id.as_str(), s);
        });
        by_id
    }

    /// Статистика узла агрегата; у узла одиночного пайплайна её нет.
    pub fn stats(&self) -> &Stats {
        self.stats.as_ref().expect("узел агрегата")
    }
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
        stats: None,
        shard_group: None,
    }
}

/// Без старта — в конец.
pub fn by_start(s: &Span) -> (bool, Option<Ms>) {
    (s.start.is_none(), s.start)
}

fn bounds(spans: &[Span]) -> (Option<Ms>, Option<Ms>) {
    let start = spans.iter().filter_map(|s| s.start).min();
    let end = spans.iter().filter_map(|s| s.end).max();
    (start, end)
}

/// Листья группы шардов; любой другой узел — сам себе лист.
pub fn leaf_ids(s: &Span) -> Vec<&str> {
    match s.kind {
        Kind::Group => s.children.iter().flat_map(leaf_ids).collect(),
        Kind::Pipeline | Kind::Stage | Kind::Job | Kind::Bridge => vec![&s.id],
    }
}

/// Порядок соседей: узел встаёт сразу за соседом, которого ждал дольше всех
/// (он же — `after`), остальные — по старту.
pub fn order_by_deps(mut children: Vec<Span>) -> Vec<Span> {
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
    Ctx {
        base_url,
        origin: raw.pipeline.created_at,
        now,
    }
    .pipeline(raw)
}

/// Общее для всего дерева: downstream отсчитывает время от корня.
struct Ctx<'a> {
    base_url: &'a str,
    origin: OffsetDateTime,
    now: OffsetDateTime,
}

/// Unix-время в мс с отброшенными долями, как `Date.parse`.
fn unix_ms(t: OffsetDateTime) -> Ms {
    // OffsetDateTime ограничен ±9999 годами — в i64 мс помещается без потерь
    (t.unix_timestamp_nanos() / 1_000_000) as Ms
}

impl Ctx<'_> {
    fn at(&self, t: OffsetDateTime) -> Ms {
        unix_ms(t) - unix_ms(self.origin)
    }

    /// Конец запущенной джобы; идущая заканчивается в `now`.
    fn end_of(&self, j: &RawJob) -> Option<Ms> {
        j.started_at
            .map(|_| self.at(j.finished_at.unwrap_or(self.now)))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    fn pipeline(&self, raw: &RawPipeline) -> Span {
        let ordered: Vec<&RawJob> = raw.jobs.iter().rev().collect();
        let id_by_name: HashMap<&str, &str> = ordered
            .iter()
            .filter(|j| !j.retried)
            .map(|j| (j.name.as_str(), j.id.as_str()))
            .collect();
        let mut jobs: Vec<Span> = ordered
            .iter()
            .filter(|j| !j.retried)
            .map(|j| self.job(j, raw, &ordered, &id_by_name))
            .collect();

        let p = &raw.pipeline;
        let stages: Vec<Span> = stage_names(&p.stages, &ordered)
            .into_iter()
            .enumerate()
            .map(|(position, stage)| {
                let id = format!("{}:{stage}", p.id);
                let members = jobs
                    .extract_if(.., |s| s.stage.as_deref() == Some(stage))
                    .collect();
                let children = group_shards(members, &id);
                let (start, end) = bounds(&children);
                Span {
                    position: Some(position),
                    start,
                    end,
                    children: order_by_deps(children),
                    ..span(id, Kind::Stage, stage.to_owned())
                }
            })
            .collect();

        let stages_end = bounds(&stages).1;
        let finished = p.finished_at.map(|t| self.at(t));
        Span {
            project: Some(raw.project.clone()),
            r#ref: Some(p.r#ref.clone()),
            start: Some(self.at(p.created_at)),
            end: match finished {
                None => stages_end,
                Some(f) => Some(f.max(stages_end.unwrap_or(f))),
            },
            status: Some(p.status.to_lowercase()),
            url: Some(self.url(&p.path)),
            children: stages,
            ..span(p.id.clone(), Kind::Pipeline, format!("#{}", p.iid))
        }
    }

    fn job(
        &self,
        j: &RawJob,
        raw: &RawPipeline,
        ordered: &[&RawJob],
        id_by_name: &HashMap<&str, &str>,
    ) -> Span {
        let children: Vec<Span> = raw
            .downstream
            .get(&j.id)
            .map(|ds| self.pipeline(ds))
            .into_iter()
            .collect();
        // bridge заканчивается не раньше своего downstream
        let end = match (self.end_of(j), children.first().and_then(|ds| ds.end)) {
            (own, None) => own,
            (own, Some(ds)) => Some(own.unwrap_or(ds).max(ds)),
        };
        Span {
            stage: Some(j.stage.clone()),
            shard_group: j.shard_group.clone(),
            start: j.started_at.map(|t| self.at(t)),
            end,
            children,
            queued: j.queued_duration.map(|q| (q * 1000.0).round() as Ms),
            status: Some(j.status.to_lowercase()),
            allow_failure: j.allow_failure,
            url: Some(self.url(&j.web_path)),
            attempts: ordered
                .iter()
                .filter(|r| r.retried && r.name == j.name)
                .map(|r| Attempt {
                    start: r.started_at.map(|t| self.at(t)),
                    end: self.end_of(r),
                    status: r.status.to_lowercase(),
                    url: self.url(&r.web_path),
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
    }
}

/// Объявленный порядок — правда GitLab; стейджи без объявления — в порядке появления джоб.
fn stage_names<'a>(declared: &'a [String], ordered: &[&'a RawJob]) -> Vec<&'a str> {
    let mut seen: Vec<&str> = vec![];
    for j in ordered {
        if !seen.contains(&j.stage.as_str()) {
            seen.push(&j.stage);
        }
    }
    let mut names: Vec<&str> = declared
        .iter()
        .map(String::as_str)
        .filter(|s| seen.contains(s))
        .collect();
    for s in seen {
        if !names.contains(&s) {
            names.push(s);
        }
    }
    names
}

/// Шарды одного ключа (явный `shard_group` или имя без суффикса: `e2e: [1]`, `e2e: [2]`)
/// сворачиваются в группу на месте первого из них.
fn group_shards(mut members: Vec<Span>, stage_id: &str) -> Vec<Span> {
    members.sort_by_key(by_start);
    let groups: Vec<String> = members
        .iter()
        .map(|s| {
            s.shard_group
                .clone()
                .unwrap_or_else(|| shard_group_name(&s.name).to_owned())
        })
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
                format!("{stage_id}:{}", groups[i]),
                Kind::Group,
                groups[i].clone(),
            )
        });
    }
    children
}
