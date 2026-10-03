mod fixtures;

use fixtures::{
    JobOpts, PipelineOpts, T0, at, clean_pipeline, job, pipeline_opts, raw_pipeline,
    sample_pipeline,
};
use pipeline_trace_core::aggregate::aggregate;
use pipeline_trace_core::insights::{
    Excess, Hotspot, HotspotKind, Stability, agg_insights, insights, retry_loss, stability_by_name,
    stability_key, stage_excess,
};
use pipeline_trace_core::model::{RawPipeline, Span, build_tree};

fn build(raw: &RawPipeline) -> Span {
    build_tree(raw, "https://h", T0)
}
fn tree() -> Span {
    build(&sample_pipeline(1))
}
fn find<'a>(s: &'a Span, name: &str) -> &'a Span {
    try_find(s, name).unwrap_or_else(|| panic!("нет узла {name}"))
}
fn try_find<'a>(s: &'a Span, name: &str) -> Option<&'a Span> {
    if s.name == name {
        return Some(s);
    }
    s.children.iter().find_map(|c| try_find(c, name))
}
fn summary(hotspots: &[Hotspot]) -> Vec<(&'static str, &str, f64)> {
    hotspots
        .iter()
        .map(|h| {
            let kind = match h.kind {
                HotspotKind::Retry { .. } | HotspotKind::RetryAgg { .. } => "retry",
                HotspotKind::Excess { .. } => "excess",
            };
            (kind, h.name.as_str(), h.saving)
        })
        .collect()
}
fn run(start: i64, end: i64) -> JobOpts {
    JobOpts {
        start: Some(start),
        end: Some(end),
        ..Default::default()
    }
}
fn failed(start: i64, end: i64) -> JobOpts {
    JobOpts {
        retried: true,
        status: "FAILED",
        ..run(start, end)
    }
}
fn stability(retried: u32, present: u32) -> Stability {
    Stability { retried, present }
}

#[test]
fn retry_loss_от_старта_первой_попытки_до_старта_успешной() {
    let t = tree();
    assert_eq!(retry_loss(find(&t, "e2e: [3]")), 529_000);
    assert_eq!(retry_loss(find(&t, "lint")), 60_000);
    assert_eq!(retry_loss(find(&t, "build:server")), 0);
}

#[test]
fn stage_excess_узкое_место_и_отрыв_от_предпоследней_джобы() {
    let t = tree();
    assert_eq!(
        stage_excess(find(&t, "build")),
        Some(Excess {
            id: "gid://gitlab/Ci::Build/build:server".into(),
            peers_end: 242_000,
            excess: 60_000.0,
        })
    );
    assert_eq!(stage_excess(find(&t, "report")), None);
}

#[test]
fn insights_ретраи_и_превышение_только_с_критического_пути_без_двойного_счёта() {
    let t = tree();
    let r = insights(&t);
    assert_eq!(
        summary(&r.hotspots),
        [
            ("retry", "e2e: [3]", 529_000.0),
            ("excess", "build:server", 60_000.0)
        ]
    );
    assert_eq!(r.hotspots[0].kind, HotspotKind::Retry { retries: 2 });
    assert_eq!(r.saving, 589_000.0);
    assert_eq!(r.total_retry_loss, 589_000.0);
    assert_eq!(r.retry_loss.len(), 2);
    assert_eq!(r.stages[&find(&t, "test").id].excess, 538_000.0);
}

fn agg_of() -> Span {
    aggregate(&[build(&sample_pipeline(1)), build(&clean_pipeline(2))])
}

#[test]
fn агрегат_retried_и_средние_retry_loss_в_stats() {
    let agg = agg_of();
    let shard = find(&agg, "e2e: [3]").stats.as_ref().unwrap();
    assert_eq!(shard.retried, 1);
    assert_eq!(shard.present, 2);
    assert_eq!(shard.retry_loss, 264_500.0);
    assert_eq!(
        find(&agg, "build:server").stats.as_ref().unwrap().retried,
        0
    );
}

#[test]
fn agg_insights_экономия_взвешена_долей_на_критическом_пути_стабильность_по_узлам() {
    let agg = agg_of();
    let r = agg_insights(&agg);
    assert_eq!(
        summary(&r.hotspots),
        [
            ("excess", "e2e: [3]", 273_500.0),
            ("retry", "e2e: [3]", 264_500.0),
            ("excess", "build:server", 60_000.0),
        ]
    );
    assert_eq!(r.stability[&find(&agg, "e2e: [3]").id], stability(1, 2));
    assert_eq!(r.total_retry_loss, 264_500.0 + 30_000.0);
}

#[test]
fn stability_by_name_метки_для_пайплайна_открытого_из_агрегата() {
    let s = stability_by_name(&agg_of());
    assert_eq!(s[&stability_key(&["test", "e2e: [3]"])], stability(1, 2));
    assert_eq!(
        s[&stability_key(&["build", "build:server"])],
        stability(0, 2)
    );
}

#[test]
fn stability_by_name_разные_lint_в_разных_стейджах_не_коллизируют() {
    let check = || {
        job(
            "check",
            "check",
            JobOpts {
                deps: vec!["build:server"],
                ..run(330, 450)
            },
        )
    };
    let with_lint_in_check = raw_pipeline(
        vec![
            job("build:server", "build", run(0, 302)),
            job("lint", "build", run(60, 240)),
            check(),
        ],
        pipeline_opts(3),
    );
    let with_retries = raw_pipeline(
        vec![
            job("build:server", "build", run(0, 302)),
            job("lint", "build", failed(0, 50)),
            job("lint", "build", run(60, 240)),
            check(),
        ],
        pipeline_opts(4),
    );
    let agg = aggregate(&[build(&with_lint_in_check), build(&with_retries)]);
    let s = stability_by_name(&agg);
    assert_eq!(s[&stability_key(&["build", "lint"])], stability(1, 2));
    assert_eq!(s[&stability_key(&["check", "check"])], stability(0, 2));
}

#[test]
fn stability_by_name_bridge_и_джобы_его_downstream_получают_путь_без_повторов() {
    let child = raw_pipeline(
        vec![job("lint", "build", run(110, 200))],
        PipelineOpts {
            created_at: at(100),
            project: "other/proj",
            ..pipeline_opts(9)
        },
    );
    let bridge = job(
        "trigger",
        "deploy",
        JobOpts {
            bridge: true,
            ..run(100, 150)
        },
    );
    let t = build(&raw_pipeline(
        vec![bridge.clone()],
        PipelineOpts {
            downstream: [(bridge.id, child)].into(),
            ..Default::default()
        },
    ));
    let mut keys: Vec<String> = stability_by_name(&aggregate(&[t])).into_keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        ["deploy / trigger", "deploy / trigger / build / lint"]
    );
}

#[test]
fn stage_excess_ожидание_зависимостей_до_старта_узкого_места_не_входит_в_превышение() {
    let t = build(&raw_pipeline(
        vec![
            job("security:code", "security", run(0, 76)),
            job("security:image", "security", run(400, 468)),
        ],
        Default::default(),
    ));
    assert_eq!(stage_excess(find(&t, "security")).unwrap().excess, 68_000.0);
}

#[test]
fn stage_excess_при_позднем_старте_с_ретраями_превышение_считается_от_первой_попытки() {
    let t = build(&raw_pipeline(
        vec![
            job("security:code", "security", run(0, 76)),
            job("security:image", "security", failed(300, 350)),
            job("security:image", "security", run(400, 468)),
        ],
        Default::default(),
    ));
    assert_eq!(
        stage_excess(find(&t, "security")).unwrap().excess,
        168_000.0
    );
}
