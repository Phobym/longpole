mod fixtures;

use std::collections::HashMap;
use std::path::Path;

use fixtures::{
    JobOpts, PipelineOpts, T0, at, clean_pipeline, job, pipeline_opts, raw_pipeline,
    sample_pipeline,
};
use pipeline_trace_core::insights::Stability;
use pipeline_trace_core::model::{RawPipeline, Span, build_tree};
use pipeline_trace_core::schema::{
    AggNode, Bar, BaseNode, Holds, Locale, Meta, Report, SingleNode, Tree,
};
use pipeline_trace_core::source::Provider;

fn build(raw: &RawPipeline) -> Span {
    build_tree(raw, "https://h", T0)
}
fn run(start: i64, end: i64, deps: &[&'static str]) -> JobOpts {
    JobOpts {
        start: Some(start),
        end: Some(end),
        deps: deps.to_vec(),
        ..Default::default()
    }
}
fn meta(label: Option<&str>) -> Meta {
    Meta {
        host: "gitlab.example.com".into(),
        project: "g/p".into(),
        label: label.map(Into::into),
        locale: Locale::Ru,
        status_counts: None,
        generated_at: "2026-09-24T07:30:00.000Z".into(),
        provider: Provider::Gitlab,
        needs_missing: false,
    }
}
fn single(raw: &RawPipeline) -> Tree<SingleNode> {
    match Report::single(meta(Some("#1")), &build(raw)) {
        Report::Single { tree, .. } => tree,
        Report::Aggregate { .. } => unreachable!(),
    }
}
fn aggregate(raws: &[RawPipeline]) -> (Tree<AggNode>, Vec<Tree<SingleNode>>) {
    let trees: Vec<Span> = raws.iter().map(build).collect();
    match Report::aggregate(meta(None), &trees) {
        Report::Aggregate { agg, trees, .. } => (agg, trees),
        Report::Single { .. } => unreachable!(),
    }
}
fn id_of<N: AsRef<BaseNode>>(tree: &Tree<N>, name: &str) -> String {
    let found: Vec<&String> = tree
        .nodes
        .iter()
        .filter(|(_, n)| n.as_ref().name == name)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "узел {name}");
    found[0].clone()
}
fn node<'a, N: AsRef<BaseNode>>(tree: &'a Tree<N>, name: &str) -> &'a N {
    &tree.nodes[&id_of(tree, name)]
}
fn base<'a, N: AsRef<BaseNode>>(tree: &'a Tree<N>, name: &str) -> &'a BaseNode {
    node(tree, name).as_ref()
}
fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|&s| s.into()).collect()
}

/// Пайплайн из `insights.test` плюс bridge с downstream.
fn rich_pipeline(i: usize) -> RawPipeline {
    let bridge = job(
        "trigger",
        "deploy",
        JobOpts {
            bridge: true,
            ..run(1400, 1450, &["report"])
        },
    );
    let downstream = raw_pipeline(
        vec![
            job("migrate", "release", run(1410, 1500, &[])),
            job("smoke", "release", run(1500, 1560, &["migrate"])),
        ],
        PipelineOpts {
            created_at: at(1400),
            project: "g/infra",
            ..pipeline_opts(100 + i)
        },
    );
    let mut raw = sample_pipeline(i);
    // джобы от новых к старым: bridge — последний
    raw.jobs.insert(0, bridge.clone());
    raw.downstream = HashMap::from([(bridge.id, downstream)]);
    raw
}

#[test]
fn single_scope_критического_пути_пайплайн_стейджи_и_downstream_без_bridge() {
    let tree = single(&rich_pipeline(1));
    let mut scopes: Vec<&str> = tree.critical.keys().map(String::as_str).collect();
    scopes.sort();
    let mut expected = vec![
        "gid://gitlab/Ci::Pipeline/1".to_owned(),
        "gid://gitlab/Ci::Pipeline/101".to_owned(),
        "gid://gitlab/Ci::Pipeline/101:release".to_owned(),
    ];
    for stage in ["build", "test", "report", "deploy"] {
        expected.push(format!("gid://gitlab/Ci::Pipeline/1:{stage}"));
    }
    expected.sort();
    assert_eq!(scopes, expected);
    assert_eq!(
        tree.critical[&tree.root].ids,
        ids(&[
            "gid://gitlab/Ci::Build/build:server",
            "gid://gitlab/Ci::Build/e2e: [3]",
            "gid://gitlab/Ci::Build/report",
            "gid://gitlab/Ci::Build/migrate",
            "gid://gitlab/Ci::Build/smoke",
            "gid://gitlab/Ci::Build/trigger",
            // группа на пути, если на пути её шард
            "gid://gitlab/Ci::Pipeline/1:test:e2e",
        ])
    );
    assert_eq!(
        tree.critical["gid://gitlab/Ci::Pipeline/1:build"].ids,
        ids(&["gid://gitlab/Ci::Build/build:server"])
    );
}

#[test]
fn single_crit_share_dependents_bar_и_insights_в_узлах() {
    let tree = single(&rich_pipeline(1));
    let group = base(&tree, "e2e");
    assert_eq!(group.crit_share, Some(1.0));
    assert_eq!(base(&tree, "lint").crit_share, Some(0.0));
    assert_eq!(base(&tree, "build").crit_share, None);
    // у группы — объединение по шардам без своих узлов
    assert_eq!(
        group.dependents,
        Some(ids(&["gid://gitlab/Ci::Build/report"]))
    );
    assert_eq!(
        base(&tree, "build:server").dependents,
        Some(ids(&[
            "gid://gitlab/Ci::Pipeline/1:test:e2e",
            "gid://gitlab/Ci::Build/e2e: [1]",
            "gid://gitlab/Ci::Build/e2e: [3]",
        ]))
    );
    assert_eq!(base(&tree, "build").dependents, None);
    assert_eq!(
        base(&tree, "lint").bar,
        Some(Bar {
            start: 60_000,
            end: 240_000
        })
    );
    assert_eq!(base(&tree, "lint").retry_loss, Some(60_000.0));
    assert_eq!(base(&tree, "build:static").retry_loss, None);
    assert_eq!(
        base(&tree, "build").excess.as_ref().map(|e| e.excess),
        Some(60_000.0)
    );
    assert_eq!(
        base(&tree, "build:server").holds,
        Some(Holds {
            stage: "gid://gitlab/Ci::Pipeline/1:build".into(),
            excess: 60_000.0
        })
    );
    assert_eq!(base(&tree, "e2e: [3]").saving, Some(529_000.0));
    assert_eq!(base(&tree, "build:server").saving, Some(60_000.0));
    assert_eq!(base(&tree, "lint").saving, None);
    // плюс smoke держит стейдж release в downstream
    assert_eq!(base(&tree, "smoke").saving, Some(60_000.0));
    assert_eq!(tree.saving, 649_000.0);
    assert_eq!(tree.total_retry_loss, 589_000.0);
    assert_eq!(tree.hotspots.len(), 3);
    // у пайплайна по ссылке меток стабильности нет
    assert_eq!(base(&tree, "lint").stability, None);
    let pipeline = &tree.nodes[&tree.root];
    assert_eq!(pipeline.project.as_deref(), Some("g/p"));
    assert_eq!(pipeline.base.parent, None);
    assert_eq!(
        base(&tree, "build").parent.as_deref(),
        Some(tree.root.as_str())
    );
}

#[test]
fn агрегат_один_scope_узлы_от_половины_пайплайнов_стабильность_деревьев_по_имени() {
    let (agg, trees) = aggregate(&[sample_pipeline(1), clean_pipeline(2)]);
    assert_eq!(agg.root, "agg");
    assert_eq!(agg.critical.keys().collect::<Vec<_>>(), ["agg"]);
    let crit = &agg.critical["agg"];
    assert!(crit.gaps.is_empty());
    assert!(crit.ids.contains(&id_of(&agg, "build:server")));
    assert!(crit.ids.contains(&id_of(&agg, "e2e")));
    assert!(!crit.ids.contains(&id_of(&agg, "lint")));
    assert_eq!(
        base(&agg, "build:server").bar,
        Some(Bar {
            start: 0,
            end: 302_000
        })
    );
    let flaky = Some(Stability {
        retried: 1,
        present: 2,
    });
    assert_eq!(base(&agg, "e2e: [3]").stability, flaky);
    assert_eq!(node(&agg, "build").position, Some(0));
    assert_eq!(base(&trees[1], "e2e: [3]").stability, flaky);
    assert_eq!(trees[0].saving, 589_000.0);
}

#[test]
fn причуда_в_агрегате_свой_stats_critical_у_стейджа_и_группы_0_группа_подсвечена_только_через_шард()
{
    // на пути по одному шарду в каждом из трёх пайплайнов: каждый < 0,5, вместе ≥ 0,5
    let t = |i: usize, slow: &'static str| {
        let shard = |name: &'static str| {
            let end = if name == slow { 200 } else { 100 };
            job(name, "test", run(10, end, &["compile"]))
        };
        raw_pipeline(
            vec![
                job("compile", "build", run(0, 10, &[])),
                shard("e2e 1/3"),
                shard("e2e 2/3"),
                shard("e2e 3/3"),
            ],
            pipeline_opts(i),
        )
    };
    let (agg, _) = aggregate(&[t(1, "e2e 1/3"), t(2, "e2e 2/3"), t(3, "e2e 3/3")]);
    let group = node(&agg, "e2e");
    assert_eq!(group.stats.critical, 0.0);
    assert_eq!(node(&agg, "test").stats.critical, 0.0);
    assert_eq!(node(&agg, "e2e 1/3").stats.critical, 1.0 / 3.0);
    assert!(group.base.crit_share.unwrap() >= 0.5);
    let ids = &agg.critical["agg"].ids;
    assert!(!ids.contains(&id_of(&agg, "e2e")));
    assert!(!ids.contains(&id_of(&agg, "test")));
    assert!(ids.contains(&id_of(&agg, "compile")));
}

#[test]
fn meta_label_null_все_пайплайны_и_locale() {
    let json = serde_json::to_value(meta(None)).unwrap();
    assert_eq!(json["label"], serde_json::Value::Null);
    assert_eq!(json["locale"], "ru");
    assert_eq!(json["statusCounts"], serde_json::Value::Null);
    assert_eq!(json["generatedAt"], "2026-09-24T07:30:00.000Z");
}

/// Эталонные отчёты для разработки фронтенда; CI проверяет, что выгрузка закоммичена.
#[test]
fn выгрузка_эталонных_report_json() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/shared/api/fixtures");
    let write = |name: &str, report: &Report| {
        let json = serde_json::to_string_pretty(report).unwrap() + "\n";
        std::fs::write(dir.join(name), json).unwrap();
    };

    write(
        "single.json",
        &Report::single(meta(Some("#1")), &build(&rich_pipeline(1))),
    );

    let trees: Vec<Span> = (1..=5)
        .map(|i| {
            build(&if i % 2 == 1 {
                rich_pipeline(i)
            } else {
                clean_pipeline(i)
            })
        })
        .collect();
    let mut m = meta(None);
    m.status_counts = Some([("SUCCESS".into(), 4), ("FAILED".into(), 1)].into());
    write("aggregate.json", &Report::aggregate(m, &trees));
}
