use longpole_core::critical_path::{Critical, Gap, critical_path};
use longpole_core::model::{Kind, Span};

fn j(id: &str, start: Option<i64>, end: Option<i64>, deps: &[&str]) -> Span {
    Span {
        start,
        end,
        deps: deps.iter().map(|&d| d.into()).collect(),
        ..Span::new(id, Kind::Job, id)
    }
}
fn node(id: &str, kind: Kind, children: Vec<Span>) -> Span {
    Span {
        children,
        ..Span::new(id, kind, id)
    }
}
fn stage(id: &str, children: Vec<Span>) -> Span {
    node(id, Kind::Stage, children)
}
fn pipe(id: &str, stages: Vec<Span>) -> Span {
    node(id, Kind::Pipeline, stages)
}
fn gap(from: &str, to: &str, ms: i64) -> Gap {
    Gap {
        from: from.into(),
        to: to.into(),
        ms,
    }
}

#[test]
fn цепочка_путь_в_хронологическом_порядке_зазоры_только_положительные() {
    let root = pipe(
        "p",
        vec![stage(
            "s",
            vec![
                j("a", Some(0), Some(10), &[]),
                j("b", Some(15), Some(20), &["a"]),
                j("c", Some(20), Some(30), &["b"]),
            ],
        )],
    );
    assert_eq!(
        critical_path(&root, "p"),
        Critical {
            ids: vec!["a".into(), "b".into(), "c".into()],
            gaps: vec![gap("a", "b", 5)],
        }
    );
}

#[test]
fn ромб_выбирается_зависимость_закончившаяся_позже() {
    let root = pipe(
        "p",
        vec![
            stage("s1", vec![j("a", Some(0), Some(10), &[])]),
            stage(
                "s2",
                vec![
                    j("b", Some(10), Some(20), &["a"]),
                    j("c", Some(10), Some(40), &["a"]),
                ],
            ),
            stage("s3", vec![j("d", Some(40), Some(50), &["b", "c"])]),
        ],
    );
    assert_eq!(critical_path(&root, "p").ids, ["a", "c", "d"]);
}

#[test]
fn не_запускавшиеся_зависимости_пропускаются_при_равном_end_берётся_первая() {
    let root = pipe(
        "p",
        vec![stage(
            "s",
            vec![
                j("x", None, None, &[]),
                j("b", Some(0), Some(10), &[]),
                j("c", Some(0), Some(10), &[]),
                j("d", Some(10), Some(20), &["x", "b", "c"]),
            ],
        )],
    );
    assert_eq!(critical_path(&root, "p").ids, ["b", "d"]);
}

#[test]
fn scope_стейджа_путь_до_его_последней_джобы() {
    let root = pipe(
        "p",
        vec![
            stage(
                "build",
                vec![
                    j("a", Some(0), Some(10), &[]),
                    j("b", Some(10), Some(30), &["a"]),
                ],
            ),
            stage("deploy", vec![j("c", Some(30), Some(90), &["b"])]),
        ],
    );
    assert_eq!(critical_path(&root, "build").ids, ["a", "b"]);
}

#[test]
fn bridge_путь_заходит_в_downstream_и_возвращается_к_зависимостям_bridge() {
    let inner = pipe(
        "p2",
        vec![stage(
            "t",
            vec![
                j("i1", Some(60), Some(70), &[]),
                j("i2", Some(75), Some(90), &["i1"]),
            ],
        )],
    );
    let br = Span {
        kind: Kind::Bridge,
        children: vec![inner],
        ..j("br", Some(55), Some(90), &["a", "noise"])
    };
    let root = pipe(
        "p",
        vec![
            stage(
                "build",
                vec![
                    j("a", Some(0), Some(50), &[]),
                    j("noise", Some(0), Some(5), &[]),
                ],
            ),
            stage("deploy", vec![br]),
        ],
    );
    assert_eq!(
        critical_path(&root, "p"),
        Critical {
            ids: vec!["a".into(), "i1".into(), "i2".into(), "br".into()],
            gaps: vec![gap("a", "i1", 10), gap("i1", "i2", 5)],
        }
    );
}

#[test]
fn scope_без_запускавшихся_джоб_даёт_пустой_путь() {
    let root = pipe("p", vec![stage("s", vec![j("m", None, None, &[])])]);
    assert_eq!(
        critical_path(&root, "s"),
        Critical {
            ids: vec![],
            gaps: vec![]
        }
    );
}
