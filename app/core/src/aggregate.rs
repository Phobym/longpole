use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::critical_path::critical_path;
use crate::insights::retry_loss;
use crate::model::{Kind, Ms, P, Sample, Span, Stats, by_start, order_by_deps};

/// Nearest-rank перцентиль.
pub fn percentile(values: &[Ms], p: u32) -> Option<Ms> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (p as usize * sorted.len()).div_ceil(100);
    sorted.get(rank.saturating_sub(1)).copied()
}

fn stat(values: &[Ms]) -> P {
    P {
        p50: percentile(values, 50),
        p90: percentile(values, 90),
    }
}

/// Один и тот же узел в разных пайплайнах.
struct Entry<'a> {
    span: &'a Span,
    tree: usize,
}

fn key_of(s: &Span) -> String {
    let kind = match s.kind {
        Kind::Pipeline => return "pipeline".into(),
        Kind::Stage => "stage",
        Kind::Group => "group",
        Kind::Job => "job",
        Kind::Bridge => "bridge",
    };
    format!("{kind}:{}", s.name)
}

/// Голоса «стейдж a раньше b» по пайплайнам, где встречаются оба.
/// Позиция в одном пайплайне не годится: в Publish-пайплайне из одного стейджа build стоит на месте 0.
fn stage_votes(entries: &[Entry]) -> HashMap<(String, String), i32> {
    let mut votes = HashMap::new();
    for e in entries {
        let names: Vec<&str> = e
            .span
            .children
            .iter()
            .filter(|c| c.kind == Kind::Stage)
            .map(|c| c.name.as_str())
            .collect();
        for (i, first) in names.iter().enumerate() {
            for second in &names[i + 1..] {
                *votes
                    .entry((first.to_string(), second.to_string()))
                    .or_default() += 1;
            }
        }
    }
    votes
}

/// Стабильная сортировка вставками с бинарным поиском, как `Array.prototype.sort` V8 на коротких
/// массивах: голоса стейджей бывают нетранзитивными, а `sort_by` std на таком порядке может паниковать.
// ponytail: O(n²) перестановок, детей у узла десятки
fn insertion_sort_by<T>(v: &mut Vec<T>, cmp: impl Fn(&T, &T) -> Ordering) {
    for i in 1..v.len() {
        let (mut lo, mut hi) = (0, i);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if cmp(&v[i], &v[mid]) == Ordering::Less {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let x = v.remove(i);
        v.insert(lo, x);
    }
}

struct Merger<'a> {
    total: usize,
    critical: Vec<HashSet<String>>,
    /// (дерево, id исходного узла) → id узла агрегата; нужен, чтобы перевести deps
    agg_id_of: HashMap<(usize, &'a str), String>,
    /// исходные deps связываемых узлов по id узла агрегата
    raw_deps: HashMap<String, Vec<(usize, &'a str)>>,
}

impl<'a> Merger<'a> {
    fn merge(&mut self, id: String, kind: Kind, name: String, entries: Vec<Entry<'a>>) -> Span {
        for e in &entries {
            self.agg_id_of.insert((e.tree, &e.span.id), id.clone());
        }
        let mut groups: Vec<(String, Kind, String, Vec<Entry<'a>>)> = vec![];
        for e in &entries {
            for c in &e.span.children {
                let key = key_of(c);
                let entry = Entry {
                    span: c,
                    tree: e.tree,
                };
                match groups.iter_mut().find(|g| g.0 == key) {
                    Some(g) => g.3.push(entry),
                    None => groups.push((key, c.kind, c.name.clone(), vec![entry])),
                }
            }
        }
        let votes = stage_votes(&entries);
        let ahead = |a: &str, b: &str| {
            let v = |x: &str, y: &str| {
                votes
                    .get(&(x.to_owned(), y.to_owned()))
                    .copied()
                    .unwrap_or(0)
            };
            v(a, b) - v(b, a)
        };
        let mut children: Vec<Span> = groups
            .into_iter()
            .map(|(key, kind, name, entries)| {
                self.merge(format!("{id}/{key}"), kind, name, entries)
            })
            .collect();
        insertion_sort_by(&mut children, |a, b| {
            if a.kind == Kind::Stage && b.kind == Kind::Stage {
                (-ahead(&a.name, &b.name))
                    .cmp(&0)
                    .then(a.position.cmp(&b.position))
            } else {
                by_start(a).cmp(&by_start(b))
            }
        });

        let ran: Vec<&Entry> = entries
            .iter()
            .filter(|e| e.span.start.is_some() && e.span.end.is_some())
            .collect();
        let times = |f: fn(&Span) -> Option<Ms>| -> Vec<Ms> {
            ran.iter().filter_map(|e| f(e.span)).collect()
        };
        let n = ran.len() as f64;
        let mean = |sum: f64| if ran.is_empty() { 0.0 } else { sum / n };
        let stats = Stats {
            present: ran.len() as u32,
            total: self.total as u32,
            start: stat(&times(|s| s.start)),
            end: stat(&times(|s| s.end)),
            duration: stat(&times(|s| Some(s.end? - s.start?))),
            queued: stat(&times(|s| s.queued)),
            retries: mean(ran.iter().map(|e| e.span.attempts.len() as f64).sum()),
            retried: ran.iter().filter(|e| !e.span.attempts.is_empty()).count() as u32,
            retry_loss: mean(ran.iter().map(|e| retry_loss(e.span) as f64).sum()),
            critical: ran
                .iter()
                .filter(|e| self.critical[e.tree].contains(&e.span.id))
                .count() as f64
                / self.total.max(1) as f64,
            samples: ran
                .iter()
                .map(|e| Sample {
                    tree: e.tree,
                    start: e.span.start.unwrap_or_default(),
                    end: e.span.end.unwrap_or_default(),
                    retries: e.span.attempts.len() as u32,
                })
                .collect(),
        };
        if matches!(kind, Kind::Job | Kind::Bridge | Kind::Group) {
            let deps = entries
                .iter()
                .flat_map(|e| e.span.deps.iter().map(|d| (e.tree, d.as_str())))
                .collect();
            self.raw_deps.insert(id.clone(), deps);
        }
        Span {
            start: stats.start.p50,
            end: stats.end.p50,
            queued: stats.queued.p50,
            position: (kind == Kind::Stage)
                .then(|| entries.iter().filter_map(|e| e.span.position).min())
                .flatten(),
            children,
            stats: Some(stats),
            ..Span::new(id, kind, name)
        }
    }

    /// deps переводятся в id агрегата после слияния: зависимость может лежать в соседнем стейдже.
    fn link(&self, s: &mut Span) {
        if let Some(raw) = self.raw_deps.get(&s.id) {
            let mut deps: Vec<String> = vec![];
            for key in raw {
                if let Some(agg) = self.agg_id_of.get(key)
                    && *agg != s.id
                    && !deps.contains(agg)
                {
                    deps.push(agg.clone());
                }
            }
            s.deps = deps;
        }
        for c in &mut s.children {
            self.link(c);
        }
    }
}

fn reorder(s: &mut Span) {
    if matches!(s.kind, Kind::Stage | Kind::Group) {
        s.children = order_by_deps(std::mem::take(&mut s.children));
    }
    s.children.iter_mut().for_each(reorder);
}

/// Агрегат пайплайнов: узлы с одинаковым путём имён сливаются, время — p50 по запускам.
pub fn aggregate(trees: &[Span]) -> Span {
    let mut m = Merger {
        total: trees.len(),
        critical: trees
            .iter()
            .map(|t| critical_path(t, &t.id).ids.into_iter().collect())
            .collect(),
        agg_id_of: HashMap::new(),
        raw_deps: HashMap::new(),
    };
    let entries = trees
        .iter()
        .enumerate()
        .map(|(tree, span)| Entry { span, tree })
        .collect();
    // имя корня («N пайплайнов») отчёт пишет сам на своём языке по stats.total
    let mut root = m.merge("agg".into(), Kind::Pipeline, String::new(), entries);
    m.link(&mut root);
    reorder(&mut root);
    root
}
