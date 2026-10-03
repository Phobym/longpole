use std::collections::HashMap;

use serde::Serialize;
use ts_rs::TS;

use crate::model::{Kind, Ms, Span};

/// Критический путь scope: джобы в хронологическом порядке и ожидания между ними.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Critical {
    pub ids: Vec<String>,
    pub gaps: Vec<Gap>,
}

/// Ожидание между соседними джобами пути.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct Gap {
    pub from: String,
    pub to: String,
    pub ms: Ms,
}

fn index<'a>(s: &'a Span, by_id: &mut HashMap<&'a str, &'a Span>) {
    by_id.insert(&s.id, s);
    for c in &s.children {
        index(c, by_id);
    }
}

/// Джобы и bridge; в downstream bridge не заходит.
pub fn leaves(s: &Span) -> Vec<&Span> {
    match s.kind {
        Kind::Job | Kind::Bridge => vec![s],
        Kind::Pipeline | Kind::Stage | Kind::Group => s.children.iter().flat_map(leaves).collect(),
    }
}

/// Закончившийся последним; при равном конце — первый.
fn latest<'a>(spans: impl IntoIterator<Item = &'a Span>) -> Option<&'a Span> {
    spans.into_iter().fold(None, |best, s| match (s.end, best) {
        (None, _) => best,
        (Some(e), Some(b)) if Some(e) <= b.end => best,
        _ => Some(s),
    })
}

/// Цепочка от последней джобы scope назад по зависимости, закончившейся позже всех.
/// bridge сначала проходит свой downstream.
pub fn critical_path(root: &Span, scope_id: &str) -> Critical {
    let mut by_id = HashMap::new();
    index(root, &mut by_id);

    fn walk<'a>(
        mut job: Option<&'a Span>,
        by_id: &HashMap<&str, &'a Span>,
        path: &mut Vec<&'a Span>,
    ) {
        while let Some(j) = job {
            path.push(j);
            if let (Kind::Bridge, Some(ds)) = (j.kind, j.children.first()) {
                walk(latest(leaves(ds)), by_id, path);
            }
            job = latest(
                j.deps
                    .iter()
                    .filter_map(|id| by_id.get(id.as_str()).copied()),
            );
        }
    }
    let mut path = vec![];
    if let Some(&scope) = by_id.get(scope_id) {
        let start = match scope.kind {
            Kind::Bridge => latest([scope]),
            _ => latest(leaves(scope)),
        };
        walk(start, &by_id, &mut path);
    }
    path.reverse();

    let gaps = path
        .windows(2)
        .filter_map(|w| {
            let (prev, next) = (w[0], w[1]);
            // на пути только закончившиеся джобы
            let ms = next.start? - prev.end?;
            (ms > 0).then(|| Gap {
                from: prev.id.clone(),
                to: next.id.clone(),
                ms,
            })
        })
        .collect();
    Critical {
        ids: path.into_iter().map(|s| s.id.clone()).collect(),
        gaps,
    }
}
