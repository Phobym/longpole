mod fixtures;

use fixtures::{JobOpts, PipelineOpts, T0, job, names, pipeline_opts, raw_pipeline, run};
use pipeline_trace_core::aggregate::{aggregate, percentile};
use pipeline_trace_core::model::{Kind, P, RawPipeline, Span, build_tree, order_by_deps};

fn build(raw: &RawPipeline) -> Span {
    build_tree(raw, "https://h", T0)
}
fn opts(i: usize) -> PipelineOpts {
    pipeline_opts(i)
}
fn after(start: i64, end: i64, deps: &[&'static str]) -> JobOpts {
    JobOpts {
        deps: deps.to_vec(),
        ..run(start, end)
    }
}
fn kind(k: Kind) -> &'static str {
    match k {
        Kind::Pipeline => "pipeline",
        Kind::Stage => "stage",
        Kind::Group => "group",
        Kind::Job => "job",
        Kind::Bridge => "bridge",
    }
}
fn child<'a>(span: &'a Span, key: &str) -> &'a Span {
    span.children
        .iter()
        .find(|c| format!("{}:{}", kind(c.kind), c.name) == key)
        .unwrap_or_else(|| panic!("нет узла {key}"))
}
fn p(p50: i64, p90: i64) -> P {
    P {
        p50: Some(p50),
        p90: Some(p90),
    }
}

fn tree(i: usize, build_end: i64, lint: bool) -> Span {
    let mut jobs = vec![
        job("prepare", "prepare", run(0, 10)),
        job("build", "build", after(10, build_end, &["prepare"])),
    ];
    if lint {
        jobs.push(job("lint", "build", after(10, 20, &["prepare"])));
    }
    build(&raw_pipeline(jobs, opts(i)))
}
fn trees() -> Vec<Span> {
    vec![tree(1, 100, true), tree(2, 300, true), tree(3, 200, false)]
}

#[test]
fn percentile_nearest_rank() {
    assert_eq!(percentile(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 50), Some(5));
    assert_eq!(percentile(&[10, 1, 9, 2, 8, 3, 7, 4, 6, 5], 90), Some(9));
    assert_eq!(percentile(&[7], 90), Some(7));
    assert_eq!(percentile(&[], 50), None);
}

#[test]
fn длина_пайплайна_и_до_конца_стейджа_перцентиль_по_пайплайнам() {
    let agg = aggregate(&trees());
    assert_eq!(agg.id, "agg");
    // имя корня («3 пайплайнов») отчёт пишет сам на своём языке по stats.total
    assert_eq!(agg.name, "");
    assert_eq!(agg.stats.as_ref().unwrap().end, p(200_000, 300_000));
    let build = child(&agg, "stage:build");
    assert_eq!(build.id, "agg/stage:build");
    assert_eq!(build.stats.as_ref().unwrap().end, p(200_000, 300_000));
    assert_eq!(
        child(build, "job:build").stats.as_ref().unwrap().duration,
        p(190_000, 290_000)
    );
}

#[test]
fn присутствие_и_доля_на_критическом_пути() {
    let agg = aggregate(&trees());
    let build = child(&agg, "stage:build");
    let lint = child(build, "job:lint").stats.as_ref().unwrap();
    assert_eq!(lint.present, 2);
    assert_eq!(lint.total, 3);
    assert_eq!(lint.critical, 0.0);
    assert_eq!(
        child(build, "job:build").stats.as_ref().unwrap().critical,
        1.0
    );
    let samples: Vec<usize> = lint.samples.iter().map(|s| s.tree).collect();
    assert_eq!(samples, [0, 1]);
}

#[test]
fn полоска_агрегата_start_и_end_p50_дети_отсортированы_по_старту() {
    let agg = aggregate(&trees());
    assert_eq!(names(&agg), ["prepare", "build"]);
    let build = child(&agg, "stage:build");
    assert_eq!(build.start, Some(10_000));
    assert_eq!(build.end, Some(200_000));
}

#[test]
fn ретраи_усредняются_по_запускам() {
    let with_retry = build(&raw_pipeline(
        vec![
            job(
                "e2e",
                "test",
                JobOpts {
                    retried: true,
                    status: "FAILED",
                    ..run(0, 5)
                },
            ),
            job("e2e", "test", run(6, 10)),
        ],
        Default::default(),
    ));
    let clean = build(&raw_pipeline(
        vec![job("e2e", "test", run(0, 10))],
        Default::default(),
    ));
    let agg = aggregate(&[with_retry, clean]);
    let e2e = child(child(&agg, "stage:test"), "job:e2e")
        .stats
        .as_ref()
        .unwrap();
    assert_eq!(e2e.retries, 0.5);
    let retries: Vec<u32> = e2e.samples.iter().map(|s| s.retries).collect();
    assert_eq!(retries, [1, 0]);
}

#[test]
fn агрегат_порядок_стейджей_по_declared_position_минимум_среди_деревьев_а_не_по_старту() {
    let t = |i| {
        build(&raw_pipeline(
            vec![
                job("prepare", "prepare", run(0, 5)),
                job("build", "build", run(10, 100)),
                job("security:code", "security", run(0, 20)),
            ],
            PipelineOpts {
                stages: vec!["prepare", "build", "security"],
                ..opts(i)
            },
        ))
    };
    assert_eq!(
        names(&aggregate(&[t(1), t(2)])),
        ["prepare", "build", "security"]
    );
}

#[test]
fn агрегат_deps_указывают_на_агрегированные_узлы_порядок_в_стейдже_по_связям() {
    let t = |i| {
        build(&raw_pipeline(
            vec![
                job("cache", "cache", run(0, 10)),
                job("server", "build", after(12, 50, &["cache"])),
                job("lint", "build", after(20, 100, &["cache"])),
                job("static", "build", after(51, 60, &["server"])),
            ],
            opts(i),
        ))
    };
    let agg = aggregate(&[t(1), t(2)]);
    let build = child(&agg, "stage:build");
    assert_eq!(names(build), ["server", "static", "lint"]);
    let static_job = child(build, "job:static");
    assert_eq!(static_job.deps, ["agg/stage:build/job:server"]);
    assert_eq!(
        static_job.after.as_deref(),
        Some("agg/stage:build/job:server")
    );
    assert_eq!(child(build, "job:lint").deps, ["agg/stage:cache/job:cache"]);
    assert_eq!(child(build, "job:lint").after, None);
    assert!(build.deps.is_empty());
}

#[test]
fn порядок_стейджей_в_агрегате_пайплайн_из_одного_стейджа_не_сдвигает_его_вперёд() {
    let full = build(&raw_pipeline(
        vec![
            job("prep", "prepare", run(0, 10)),
            job("npm", "cache", run(10, 20)),
            job("srv", "build", run(20, 30)),
            job("scan", "security", run(0, 5)),
        ],
        PipelineOpts {
            stages: vec!["prepare", "cache", "build", "security"],
            ..opts(1)
        },
    ));
    let publish = build(&raw_pipeline(
        vec![job("ver", "build", run(0, 60))],
        PipelineOpts {
            stages: vec!["build"],
            ..opts(2)
        },
    ));
    assert_eq!(
        names(&aggregate(&[publish.clone(), full, publish])),
        ["prepare", "cache", "build", "security"]
    );
}

// order.test: порядок соседей по связям

fn s(id: &str, start: i64, end: i64, deps: &[&str]) -> Span {
    Span {
        start: Some(start),
        end: Some(end),
        deps: deps.iter().map(|&d| d.into()).collect(),
        ..Span::new(id, Kind::Job, id)
    }
}
fn ids(list: &[Span]) -> Vec<&str> {
    list.iter().map(|x| x.id.as_str()).collect()
}
fn afters(list: &[Span]) -> Vec<Option<&str>> {
    list.iter().map(|x| x.after.as_deref()).collect()
}

#[test]
fn зависимая_джоба_идёт_сразу_после_своей_зависимости() {
    let out = order_by_deps(vec![
        s("server", 0, 50, &[]),
        s("lint", 5, 60, &[]),
        s("static", 55, 70, &["server"]),
    ]);
    assert_eq!(ids(&out), ["server", "static", "lint"]);
    assert_eq!(afters(&out), [None, Some("server"), None]);
}

#[test]
fn при_нескольких_зависимостях_родитель_закончившаяся_последней() {
    let out = order_by_deps(vec![
        s("a", 0, 30, &[]),
        s("b", 0, 10, &[]),
        s("c", 31, 40, &["b", "a"]),
    ]);
    assert_eq!(ids(&out), ["a", "c", "b"]);
    assert_eq!(out[1].after.as_deref(), Some("a"));
}

#[test]
fn при_равном_end_родитель_сосед_стартовавший_раньше() {
    let out = order_by_deps(vec![
        s("a", 0, 10, &[]),
        s("b", 1, 10, &[]),
        s("c", 11, 20, &["b", "a"]),
    ]);
    assert_eq!(ids(&out), ["a", "c", "b"]);
}

#[test]
fn зависимость_вне_соседей_на_порядок_не_влияет() {
    let out = order_by_deps(vec![s("x", 10, 20, &["elsewhere"]), s("y", 0, 5, &[])]);
    assert_eq!(ids(&out), ["y", "x"]);
    assert_eq!(out[1].after, None);
}

#[test]
fn группа_шардов_одна_единица_её_ставят_после_зависимости_и_после_неё_зависящих_от_шардов() {
    let group = Span {
        kind: Kind::Group,
        children: vec![s("g1", 60, 80, &["server"]), s("g2", 61, 90, &["server"])],
        ..s("g", 60, 90, &["server"])
    };
    let out = order_by_deps(vec![
        s("server", 0, 50, &[]),
        s("lint", 5, 60, &[]),
        group,
        s("rep", 91, 95, &["g1", "g2"]),
    ]);
    assert_eq!(ids(&out), ["server", "g", "rep", "lint"]);
    assert_eq!(afters(&out), [None, Some("server"), Some("g"), None]);
}

#[test]
fn цикл_зависимостей_не_теряет_и_не_дублирует_узлы() {
    let out = order_by_deps(vec![s("a", 0, 10, &["b"]), s("b", 0, 20, &["a"])]);
    let mut got = ids(&out);
    got.sort();
    assert_eq!(got, ["a", "b"]);
}

#[test]
fn шарды_раскрытой_группы_сливаются_по_имени_в_стейдже() {
    let t = |i: usize| {
        build(&raw_pipeline(
            vec![
                job("e2e 1/2", "test", run(0, 10)),
                job("e2e 2/2", "test", run(1, 12)),
            ],
            opts(i),
        ))
    };
    let agg = aggregate(&[t(1), t(2)]);
    let stage = child(&agg, "stage:test");
    assert_eq!(stage.children.len(), 2);
    for key in ["job:e2e 1/2", "job:e2e 2/2"] {
        assert_eq!(child(stage, key).stats().present, 2);
    }
}
