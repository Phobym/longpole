use std::collections::{HashMap, HashSet};

use serde::Serialize;
use ts_rs::TS;

use crate::critical_path::{critical_path, leaves};
use crate::model::{Kind, Ms, Span};

/// Меньшая экономия в «Куда направить силы» не попадает.
pub const MIN_SAVING_MS: f64 = 10_000.0;
const MAX_HOTSPOTS: usize = 3;

/// Узкое место стейджа: конец его последней джобы правее конца остальных.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Excess {
    pub id: String,
    pub peers_end: Ms,
    pub excess: f64,
}

/// Ретраи узла по пайплайнам агрегата.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Stability {
    pub retried: u32,
    pub present: u32,
}

/// Пункт «Куда направить силы».
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Hotspot {
    pub id: String,
    pub name: String,
    pub saving: f64,
    #[serde(flatten)]
    pub kind: HotspotKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[ts(export)]
pub enum HotspotKind {
    /// ретраи в одном пайплайне
    Retry {
        retries: u32,
    },
    /// ретраи по пайплайнам агрегата
    #[serde(rename = "retry")]
    RetryAgg {
        retried: u32,
        present: u32,
    },
    Excess {
        stage: String,
        excess: f64,
    },
}

/// Выводы по дереву пайплайна или агрегата.
#[derive(Debug, Default)]
pub struct Insights {
    /// превышение по id стейджа
    pub stages: HashMap<String, Excess>,
    /// потери на ретраях по id листа, только ненулевые
    pub retry_loss: HashMap<String, f64>,
    pub total_retry_loss: f64,
    pub hotspots: Vec<Hotspot>,
    /// сумма экономии hotspots
    pub saving: f64,
    /// только агрегат: по id листа
    pub stability: HashMap<String, Stability>,
}

fn walk<'a>(s: &'a Span, f: &mut impl FnMut(&'a Span)) {
    f(s);
    for c in &s.children {
        walk(c, f);
    }
}

fn index(root: &Span) -> HashMap<&str, &Span> {
    let mut by_id = HashMap::new();
    walk(root, &mut |s| {
        by_id.insert(s.id.as_str(), s);
    });
    by_id
}

fn rank(mut candidates: Vec<Hotspot>) -> Vec<Hotspot> {
    candidates.retain(|c| c.saving >= MIN_SAVING_MS);
    candidates.sort_by(|a, b| b.saving.total_cmp(&a.saving));
    candidates.truncate(MAX_HOTSPOTS);
    candidates
}

/// Столько пайплайн ждал из-за упавших попыток джобы.
pub fn retry_loss(span: &Span) -> Ms {
    let first = span.attempts.iter().filter_map(|a| a.start).min();
    match (first, span.start) {
        (Some(first), Some(start)) => (start - first).max(0),
        _ => 0,
    }
}

fn excess_by(
    stage: &Span,
    end_of: impl Fn(&Span) -> Option<Ms>,
    start_of: impl Fn(&Span) -> f64,
) -> Option<Excess> {
    let mut ran: Vec<(&Span, Ms)> = leaves(stage)
        .into_iter()
        .filter_map(|s| end_of(s).map(|e| (s, e)))
        .collect();
    ran.sort_by(|a, b| b.1.cmp(&a.1));
    let [(last, last_end), (_, peers_end), ..] = ran[..] else {
        return None;
    };
    // ожидание зависимостей до первой попытки — не превышение самой джобы
    let excess = last_end as f64 - (peers_end as f64).max(start_of(last));
    (excess > 0.0).then(|| Excess {
        id: last.id.clone(),
        peers_end,
        excess,
    })
}

/// Превышение стейджа в одном пайплайне: от первой попытки узкого места.
pub fn stage_excess(stage: &Span) -> Option<Excess> {
    excess_by(
        stage,
        |s| s.end,
        // у листа с концом старт есть
        |s| (s.start.unwrap_or_default() - retry_loss(s)) as f64,
    )
}

/// Конец агрегированного узла совпадает с концом его сплошной полоски в отчёте.
pub fn agg_end(s: &Span) -> Option<Ms> {
    let st = s.stats.as_ref()?;
    Some(st.start.p50? + st.duration.p50?)
}

fn stats(s: &Span) -> &crate::aggregate::Stats {
    s.stats.as_ref().expect("узел агрегата")
}

/// Выводы по пайплайну: в hotspots только узлы его критического пути.
pub fn insights(tree: &Span) -> Insights {
    let crit: HashSet<String> = critical_path(tree, &tree.id).ids.into_iter().collect();
    let by_id = index(tree);
    let mut out = Insights::default();
    let mut candidates = vec![];
    walk(tree, &mut |s| match s.kind {
        Kind::Job | Kind::Bridge => {
            let loss = retry_loss(s) as f64;
            if loss == 0.0 {
                return;
            }
            out.retry_loss.insert(s.id.clone(), loss);
            out.total_retry_loss += loss;
            if crit.contains(&s.id) {
                candidates.push(Hotspot {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    saving: loss,
                    kind: HotspotKind::Retry {
                        retries: s.attempts.len() as u32,
                    },
                });
            }
        }
        Kind::Stage => {
            let Some(ex) = stage_excess(s) else { return };
            if crit.contains(&ex.id) {
                let bottleneck = by_id[ex.id.as_str()];
                // ретраи узкого места уже стали отдельным пунктом — вычитаем, чтобы не считать дважды
                candidates.push(Hotspot {
                    id: ex.id.clone(),
                    name: bottleneck.name.clone(),
                    saving: (ex.excess - retry_loss(bottleneck) as f64).max(0.0),
                    kind: HotspotKind::Excess {
                        stage: s.name.clone(),
                        excess: ex.excess,
                    },
                });
            }
            out.stages.insert(s.id.clone(), ex);
        }
        Kind::Pipeline | Kind::Group => {}
    });
    out.hotspots = rank(candidates);
    out.saving = out.hotspots.iter().map(|h| h.saving).sum();
    out
}

/// Выводы по агрегату: экономия взвешена долей пайплайнов, где узел на критическом пути.
pub fn agg_insights(agg: &Span) -> Insights {
    let by_id = index(agg);
    let mut out = Insights::default();
    let mut candidates = vec![];
    walk(agg, &mut |s| match s.kind {
        Kind::Job | Kind::Bridge => {
            let st = stats(s);
            out.stability.insert(
                s.id.clone(),
                Stability {
                    retried: st.retried,
                    present: st.present,
                },
            );
            let loss = st.retry_loss;
            if loss == 0.0 {
                return;
            }
            out.retry_loss.insert(s.id.clone(), loss);
            out.total_retry_loss += loss;
            if st.critical > 0.0 {
                candidates.push(Hotspot {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    saving: loss * st.critical,
                    kind: HotspotKind::RetryAgg {
                        retried: st.retried,
                        present: st.present,
                    },
                });
            }
        }
        Kind::Stage => {
            let first_start = |n: &Span| {
                let st = stats(n);
                st.start.p50.unwrap_or_default() as f64 - st.retry_loss
            };
            let Some(ex) = excess_by(s, agg_end, first_start) else {
                return;
            };
            let bottleneck = by_id[ex.id.as_str()];
            let st = stats(bottleneck);
            if st.critical != 0.0 {
                candidates.push(Hotspot {
                    id: ex.id.clone(),
                    name: bottleneck.name.clone(),
                    saving: (ex.excess - st.retry_loss).max(0.0) * st.critical,
                    kind: HotspotKind::Excess {
                        stage: s.name.clone(),
                        excess: ex.excess,
                    },
                });
            }
            out.stages.insert(s.id.clone(), ex);
        }
        Kind::Pipeline | Kind::Group => {}
    });
    out.hotspots = rank(candidates);
    out.saving = out.hotspots.iter().map(|h| h.saving).sum();
    out
}

/// Ключ метки стабильности: имена предков-стейджей и bridge плюс имя узла,
/// одинаковый у дерева пайплайна и у агрегата.
pub fn stability_key(names: &[&str]) -> String {
    names.join(" / ")
}

/// Метки стабильности агрегата для пайплайнов, открытых из него.
pub fn stability_by_name(agg: &Span) -> HashMap<String, Stability> {
    fn go<'a>(s: &'a Span, ancestors: &mut Vec<&'a str>, out: &mut HashMap<String, Stability>) {
        let leaf = matches!(s.kind, Kind::Job | Kind::Bridge);
        if leaf {
            let st = stats(s);
            ancestors.push(&s.name);
            out.insert(
                stability_key(ancestors),
                Stability {
                    retried: st.retried,
                    present: st.present,
                },
            );
            ancestors.pop();
        }
        let named = matches!(s.kind, Kind::Stage | Kind::Bridge);
        if named {
            ancestors.push(&s.name);
        }
        for c in &s.children {
            go(c, ancestors, out);
        }
        if named {
            ancestors.pop();
        }
    }
    let mut out = HashMap::new();
    go(agg, &mut vec![], &mut out);
    out
}
