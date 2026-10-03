//! Схема JSON отчёта (спека, § 4): нормализованное дерево, всё производное посчитано здесь.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use ts_rs::TS;

use crate::aggregate::{Stats, aggregate};
use crate::critical_path::{Critical, critical_path};
use crate::insights::{
    Excess, Hotspot, Insights, Stability, agg_end, agg_insights, insights, stability_by_name,
    stability_key,
};
use crate::model::{Attempt, Kind, Ms, Span, leaf_ids};

/// Порог подсветки узла агрегата на критическом пути.
const AGG_CRITICAL: f64 = 0.5;

#[derive(Debug, Serialize, TS)]
#[serde(tag = "mode", rename_all = "lowercase")]
#[ts(export)]
pub enum Report {
    Single {
        meta: Meta,
        tree: Tree<SingleNode>,
    },
    Aggregate {
        meta: Meta,
        agg: Tree<AggNode>,
        trees: Vec<Tree<SingleNode>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Locale {
    Ru,
    En,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Meta {
    pub host: String,
    pub project: String,
    /// `None` — «все пайплайны», текст подставляет отчёт на своём языке
    pub label: Option<String>,
    pub locale: Locale,
    pub status_counts: Option<BTreeMap<String, u32>>,
    /// ISO 8601
    pub generated_at: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Tree<N> {
    pub root: String,
    pub nodes: BTreeMap<String, N>,
    /// По scope: в single — пайплайн, его стейджи, downstream-пайплайны и их стейджи,
    /// в агрегате — только корень
    pub critical: BTreeMap<String, Critical>,
    pub hotspots: Vec<Hotspot>,
    pub total_retry_loss: f64,
    pub saving: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Bar {
    pub start: Ms,
    pub end: Ms,
}

/// Узел держит стейдж: его хвост правее остальных джоб стейджа.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Holds {
    pub stage: String,
    pub excess: f64,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BaseNode {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub deps: Vec<String>,
    pub after: Option<String>,
    pub bar: Option<Bar>,
    /// job, bridge, group: кто ждёт узел
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dependents: Option<Vec<String>>,
    /// job, bridge, group: single — 0 или 1, агрегат — сумма долей шардов, не больше 1
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub crit_share: Option<f64>,
    /// стейдж
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub excess: Option<Excess>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub holds: Option<Holds>,
    /// лист, > 0
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub retry_loss: Option<f64>,
    /// лист агрегата или пайплайна из агрегата
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stability: Option<Stability>,
    /// сумма его hotspots
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub saving: Option<f64>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SingleNode {
    #[serde(flatten)]
    pub base: BaseNode,
    pub start: Option<Ms>,
    pub end: Option<Ms>,
    pub queued: Option<Ms>,
    pub status: Option<String>,
    pub allow_failure: bool,
    pub url: Option<String>,
    pub attempts: Vec<Attempt>,
    /// пайплайн
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub r#ref: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AggNode {
    #[serde(flatten)]
    pub base: BaseNode,
    /// стейдж
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub position: Option<usize>,
    pub stats: Stats,
}

impl AsRef<BaseNode> for SingleNode {
    fn as_ref(&self) -> &BaseNode {
        &self.base
    }
}

impl AsRef<BaseNode> for AggNode {
    fn as_ref(&self) -> &BaseNode {
        &self.base
    }
}

impl Report {
    pub fn single(meta: Meta, tree: &Span) -> Report {
        Report::Single {
            meta,
            tree: single_tree(tree, None),
        }
    }

    pub fn aggregate(meta: Meta, trees: &[Span]) -> Report {
        let agg = aggregate(trees);
        let by_name = stability_by_name(&agg);
        Report::Aggregate {
            meta,
            agg: agg_tree(&agg),
            trees: trees
                .iter()
                .map(|t| single_tree(t, Some(&by_name)))
                .collect(),
        }
    }
}

fn is_leaf(s: &Span) -> bool {
    matches!(s.kind, Kind::Job | Kind::Bridge)
}

fn is_linkable(s: &Span) -> bool {
    matches!(s.kind, Kind::Job | Kind::Bridge | Kind::Group)
}

/// Узлы в порядке обхода с родителем и именами предков-стейджей и bridge.
fn flatten<'a>(
    s: &'a Span,
    parent: Option<&'a Span>,
    path: &mut Vec<&'a str>,
    out: &mut Vec<Visit<'a>>,
) {
    out.push(Visit {
        span: s,
        parent,
        path: path.clone(),
    });
    let named = matches!(s.kind, Kind::Stage | Kind::Bridge);
    if named {
        path.push(&s.name);
    }
    for c in &s.children {
        flatten(c, Some(s), path, out);
    }
    if named {
        path.pop();
    }
}

struct Visit<'a> {
    span: &'a Span,
    parent: Option<&'a Span>,
    /// имена предков-стейджей и bridge — ключ стабильности
    path: Vec<&'a str>,
}

/// Группа на пути, если на пути хотя бы один её шард.
fn with_groups(mut crit: Critical, visits: &[Visit]) -> Critical {
    for v in visits.iter().filter(|v| v.span.kind == Kind::Group) {
        if leaf_ids(v.span)
            .iter()
            .any(|id| crit.ids.iter().any(|c| c == id))
        {
            crit.ids.push(v.span.id.clone());
        }
    }
    crit
}

/// Общая часть узла; `crit_share`, `bar` и `stability` зависят от режима.
struct Common<'a> {
    visits: Vec<Visit<'a>>,
    ins: Insights,
    /// id узла → кто от него зависит, в порядке обхода
    waiting: HashMap<&'a str, Vec<usize>>,
    /// id узла → стейдж, который он держит
    holds: HashMap<String, Holds>,
}

impl<'a> Common<'a> {
    fn new(root: &'a Span, ins: Insights) -> Self {
        let mut visits = vec![];
        flatten(root, None, &mut vec![], &mut visits);
        let mut waiting: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut holds = HashMap::new();
        for (i, v) in visits.iter().enumerate() {
            for d in &v.span.deps {
                waiting.entry(d.as_str()).or_default().push(i);
            }
            if let Some(ex) = ins.stages.get(&v.span.id) {
                holds.entry(ex.id.clone()).or_insert(Holds {
                    stage: v.span.id.clone(),
                    excess: ex.excess,
                });
            }
        }
        Common {
            visits,
            ins,
            waiting,
            holds,
        }
    }

    /// Кто ждёт узел или его шарды, без самих шардов.
    fn dependents(&self, s: &Span) -> Vec<String> {
        let own = leaf_ids(s);
        let own = |id: &str| id == s.id || own.contains(&id);
        let mut found: Vec<usize> = [s.id.as_str()]
            .into_iter()
            .chain(leaf_ids(s))
            .flat_map(|id| self.waiting.get(id).into_iter().flatten().copied())
            .filter(|&i| !own(&self.visits[i].span.id))
            .collect();
        found.sort_unstable();
        found.dedup();
        found
            .into_iter()
            .map(|i| self.visits[i].span.id.clone())
            .collect()
    }

    fn base(
        &self,
        v: &Visit,
        crit_share: Option<f64>,
        bar: Option<Bar>,
        stability: Option<Stability>,
    ) -> BaseNode {
        let s = v.span;
        let saving: f64 = self
            .ins
            .hotspots
            .iter()
            .filter(|h| h.id == s.id)
            .map(|h| h.saving)
            .sum();
        BaseNode {
            id: s.id.clone(),
            kind: s.kind,
            name: s.name.clone(),
            parent: v.parent.map(|p| p.id.clone()),
            children: s.children.iter().map(|c| c.id.clone()).collect(),
            deps: s.deps.clone(),
            after: s.after.clone(),
            bar,
            dependents: is_linkable(s).then(|| self.dependents(s)),
            crit_share: crit_share.filter(|_| is_linkable(s)),
            excess: self.ins.stages.get(&s.id).cloned(),
            holds: self.holds.get(&s.id).cloned(),
            retry_loss: self.ins.retry_loss.get(&s.id).copied(),
            stability: stability.filter(|_| is_leaf(s)),
            saving: self
                .ins
                .hotspots
                .iter()
                .any(|h| h.id == s.id)
                .then_some(saving),
        }
    }

    fn tree<N>(
        self,
        root: &Span,
        critical: BTreeMap<String, Critical>,
        nodes: BTreeMap<String, N>,
    ) -> Tree<N> {
        Tree {
            root: root.id.clone(),
            nodes,
            critical,
            total_retry_loss: self.ins.total_retry_loss,
            saving: self.ins.saving,
            hotspots: self.ins.hotspots,
        }
    }
}

fn single_tree(root: &Span, by_name: Option<&HashMap<String, Stability>>) -> Tree<SingleNode> {
    let c = Common::new(root, insights(root));
    let critical: BTreeMap<String, Critical> = c
        .visits
        .iter()
        .filter(|v| matches!(v.span.kind, Kind::Pipeline | Kind::Stage))
        .map(|v| {
            let crit = with_groups(critical_path(root, &v.span.id), &c.visits);
            (v.span.id.clone(), crit)
        })
        .collect();
    let full = &critical[&root.id].ids;
    let nodes = c
        .visits
        .iter()
        .map(|v| {
            let s = v.span;
            let on_path = leaf_ids(s).iter().any(|id| full.iter().any(|c| c == id));
            let bar = match (s.start, s.end) {
                (Some(start), Some(end)) => Some(Bar { start, end }),
                _ => None,
            };
            let stability = by_name.and_then(|m| {
                let mut names = v.path.clone();
                names.push(&s.name);
                m.get(&stability_key(&names)).copied()
            });
            let node = SingleNode {
                base: c.base(v, Some(if on_path { 1.0 } else { 0.0 }), bar, stability),
                start: s.start,
                end: s.end,
                queued: s.queued,
                status: s.status.clone(),
                allow_failure: s.allow_failure,
                url: s.url.clone(),
                attempts: s.attempts.clone(),
                project: s.project.clone(),
                r#ref: s.r#ref.clone(),
            };
            (s.id.clone(), node)
        })
        .collect();
    c.tree(root, critical, nodes)
}

fn agg_tree(root: &Span) -> Tree<AggNode> {
    let c = Common::new(root, agg_insights(root));
    let ids = c
        .visits
        .iter()
        .filter(|v| v.span.stats().critical >= AGG_CRITICAL)
        .map(|v| v.span.id.clone())
        .collect();
    let crit = with_groups(Critical { ids, gaps: vec![] }, &c.visits);
    let index: HashMap<&str, &Span> = c
        .visits
        .iter()
        .map(|v| (v.span.id.as_str(), v.span))
        .collect();
    let nodes = c
        .visits
        .iter()
        .map(|v| {
            let s = v.span;
            // шарды группы лежат на пути по одному, поэтому доли складываются
            let share: f64 = leaf_ids(s)
                .iter()
                .filter_map(|id| index.get(id))
                .map(|n| n.stats().critical)
                .sum();
            let bar = s
                .stats()
                .start
                .p50
                .zip(agg_end(s))
                .map(|(start, end)| Bar { start, end });
            let stability = c.ins.stability.get(&s.id).copied();
            let node = AggNode {
                base: c.base(v, Some(share.min(1.0)), bar, stability),
                position: s.position,
                stats: s.stats().clone(),
            };
            (s.id.clone(), node)
        })
        .collect();
    c.tree(root, BTreeMap::from([(root.id.clone(), crit)]), nodes)
}
