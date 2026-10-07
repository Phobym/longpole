mod fixtures;

use std::collections::HashMap;

use fixtures::{JobOpts, PipelineOpts, T0, at, find, job, names, raw_pipeline};
use pipeline_trace_core::model::{
    Attempt, Kind, RawJob, RawPipeline, Span, build_tree, shard_group_name,
};
use time::{Duration, OffsetDateTime};

const BASE_URL: &str = "https://h";

fn build(raw: &RawPipeline) -> Span {
    build_tree(raw, BASE_URL, OffsetDateTime::now_utc())
}

#[test]
fn стейджи_идут_в_порядке_запуска_границы_стейджа_по_его_джобам() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "prepare:checksum",
                "prepare",
                JobOpts {
                    start: Some(0),
                    end: Some(60),
                    ..Default::default()
                },
            ),
            job(
                "cache:npm",
                "cache",
                JobOpts {
                    start: Some(5),
                    end: Some(120),
                    ..Default::default()
                },
            ),
            job(
                "lint",
                "build",
                JobOpts {
                    start: Some(125),
                    end: Some(200),
                    deps: vec!["cache:npm"],
                    ..Default::default()
                },
            ),
            job(
                "build:server",
                "build",
                JobOpts {
                    start: Some(130),
                    end: Some(300),
                    deps: vec!["cache:npm"],
                    queued: 2.0,
                    ..Default::default()
                },
            ),
            job(
                "deploy",
                "deploy",
                JobOpts {
                    status: "MANUAL",
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts {
            status: "MANUAL",
            ..Default::default()
        },
    ));

    assert_eq!(tree.kind, Kind::Pipeline);
    assert_eq!(tree.name, "#1");
    assert_eq!(tree.start, Some(0));
    assert_eq!(tree.end, Some(300_000));
    assert_eq!(tree.url.as_deref(), Some("https://h/g/p/-/pipelines/1"));
    assert_eq!(names(&tree), ["prepare", "cache", "build", "deploy"]);

    let build = find(&tree, "build");
    assert_eq!(build.kind, Kind::Stage);
    assert_eq!(build.start, Some(125_000));
    assert_eq!(build.end, Some(300_000));
    assert_eq!(names(build), ["lint", "build:server"]);

    let server = find(&tree, "build:server");
    assert_eq!(server.queued, Some(2000));
    assert_eq!(server.stage.as_deref(), Some("build"));
    assert_eq!(server.status.as_deref(), Some("success"));
    assert_eq!(server.deps, ["gid://gitlab/Ci::Build/cache:npm"]);
}

#[test]
fn порядок_стейджей_по_объявлению_в_пайплайне_а_не_по_старту_джоб_без_stages_старый_порядок() {
    let jobs = || {
        vec![
            job(
                "prepare",
                "prepare",
                JobOpts {
                    start: Some(0),
                    end: Some(5),
                    ..Default::default()
                },
            ),
            job(
                "build",
                "build",
                JobOpts {
                    start: Some(10),
                    end: Some(100),
                    ..Default::default()
                },
            ),
            job(
                "security:code",
                "security",
                JobOpts {
                    start: Some(0),
                    end: Some(20),
                    ..Default::default()
                },
            ),
        ]
    };
    let with_stages = build(&raw_pipeline(
        jobs(),
        PipelineOpts {
            stages: vec!["prepare", "build", "security"],
            ..Default::default()
        },
    ));
    let positions: Vec<_> = with_stages
        .children
        .iter()
        .map(|s| (s.name.as_str(), s.position))
        .collect();
    assert_eq!(
        positions,
        [
            ("prepare", Some(0)),
            ("build", Some(1)),
            ("security", Some(2))
        ]
    );

    // объявленный порядок расходится с порядком появления; стейдж без джоб пропускается
    let reordered = build(&raw_pipeline(
        jobs(),
        PipelineOpts {
            stages: vec!["security", "empty", "prepare"],
            ..Default::default()
        },
    ));
    assert_eq!(names(&reordered), ["security", "prepare", "build"]);

    let without_stages = build(&raw_pipeline(jobs(), PipelineOpts::default()));
    assert_eq!(names(&without_stages), ["prepare", "build", "security"]);
}

#[test]
fn manual_джоба_без_старта_остаётся_в_дереве_без_времени_и_не_влияет_на_стейдж() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "a",
                "build",
                JobOpts {
                    start: Some(0),
                    end: Some(10),
                    ..Default::default()
                },
            ),
            job(
                "deploy",
                "build",
                JobOpts {
                    status: "MANUAL",
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    let deploy = find(&tree, "deploy");
    assert_eq!(deploy.start, None);
    assert_eq!(deploy.end, None);
    assert_eq!(find(&tree, "build").end, Some(10_000));
}

#[test]
fn ретраи_уходят_в_attempts_deps_указывают_на_актуальную_попытку() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "e2e",
                "test",
                JobOpts {
                    start: Some(10),
                    end: Some(20),
                    status: "FAILED",
                    retried: true,
                    ..Default::default()
                },
            ),
            job(
                "e2e",
                "test",
                JobOpts {
                    start: Some(30),
                    end: Some(40),
                    ..Default::default()
                },
            ),
            job(
                "report",
                "reports",
                JobOpts {
                    start: Some(41),
                    end: Some(45),
                    deps: vec!["e2e", "missing"],
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    let e2e = find(&tree, "e2e");
    assert_eq!(e2e.start, Some(30_000));
    assert_eq!(
        e2e.attempts,
        [Attempt {
            start: Some(10_000),
            end: Some(20_000),
            status: "failed".into(),
            url: "https://h/g/p/-/jobs/e2e".into()
        }]
    );
    assert_eq!(find(&tree, "report").deps, ["gid://gitlab/Ci::Build/e2e"]);
}

#[test]
fn шарды_одного_стейджа_объединяются_в_group_одиночная_джоба_нет() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "e2e: [1]",
                "test",
                JobOpts {
                    start: Some(0),
                    end: Some(50),
                    ..Default::default()
                },
            ),
            job(
                "e2e: [2]",
                "test",
                JobOpts {
                    start: Some(5),
                    end: Some(80),
                    ..Default::default()
                },
            ),
            job(
                "solo",
                "test",
                JobOpts {
                    start: Some(1),
                    end: Some(2),
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    let stage = find(&tree, "test");
    let kinds: Vec<_> = stage
        .children
        .iter()
        .map(|s| (s.kind, s.name.as_str()))
        .collect();
    assert_eq!(kinds, [(Kind::Group, "e2e"), (Kind::Job, "solo")]);
    let group = &stage.children[0];
    assert_eq!(group.start, Some(0));
    assert_eq!(group.end, Some(80_000));
    assert_eq!(names(group), ["e2e: [1]", "e2e: [2]"]);
}

#[test]
fn джоба_отменённая_до_старта_не_получает_конец_и_не_растягивает_стейдж() {
    let canceled = RawJob {
        finished_at: Some(at(500)),
        ..job(
            "late",
            "build",
            JobOpts {
                status: "CANCELED",
                ..Default::default()
            },
        )
    };
    let tree = build(&raw_pipeline(
        vec![
            job(
                "a",
                "build",
                JobOpts {
                    start: Some(0),
                    end: Some(10),
                    ..Default::default()
                },
            ),
            canceled,
        ],
        PipelineOpts::default(),
    ));
    assert_eq!(find(&tree, "late").end, None);
    assert_eq!(find(&tree, "build").end, Some(10_000));
    assert_eq!(tree.end, Some(10_000));
}

#[test]
fn downstream_вложен_в_bridge_время_от_корня_bridge_заканчивается_вместе_с_ним() {
    let child = raw_pipeline(
        vec![job(
            "inner",
            "test",
            JobOpts {
                start: Some(110),
                end: Some(200),
                ..Default::default()
            },
        )],
        PipelineOpts {
            id: "gid://gitlab/Ci::Pipeline/2",
            iid: "7",
            created_at: at(100),
            project: "other/proj",
            ..Default::default()
        },
    );
    let bridge = job(
        "trigger",
        "deploy",
        JobOpts {
            bridge: true,
            start: Some(100),
            end: Some(150),
            ..Default::default()
        },
    );
    let downstream = HashMap::from([(bridge.id.clone(), child)]);
    let tree = build(&raw_pipeline(
        vec![bridge],
        PipelineOpts {
            downstream,
            ..Default::default()
        },
    ));
    let span = find(&tree, "trigger");
    assert_eq!(span.kind, Kind::Bridge);
    assert_eq!(span.end, Some(200_000));
    let ds = &span.children[0];
    assert_eq!(ds.kind, Kind::Pipeline);
    assert_eq!(ds.start, Some(100_000));
    assert_eq!(
        ds.url.as_deref(),
        Some("https://h/other/proj/-/pipelines/7")
    );
    assert_eq!(find(ds, "inner").start, Some(110_000));
    assert_eq!(tree.end, Some(200_000));
}

#[test]
fn идущая_джоба_заканчивается_в_now() {
    let tree = build_tree(
        &raw_pipeline(
            vec![job(
                "a",
                "build",
                JobOpts {
                    start: Some(10),
                    status: "RUNNING",
                    ..Default::default()
                },
            )],
            PipelineOpts::default(),
        ),
        BASE_URL,
        T0 + Duration::seconds(50),
    );
    assert_eq!(find(&tree, "a").end, Some(50_000));
}

#[test]
fn в_стейдже_джоба_стоит_после_соседа_которого_ждала_группа_получает_deps_шардов() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "cache:npm",
                "cache",
                JobOpts {
                    start: Some(0),
                    end: Some(120),
                    ..Default::default()
                },
            ),
            job(
                "build:server",
                "build",
                JobOpts {
                    start: Some(130),
                    end: Some(300),
                    deps: vec!["cache:npm"],
                    ..Default::default()
                },
            ),
            job(
                "lint",
                "build",
                JobOpts {
                    start: Some(140),
                    end: Some(200),
                    deps: vec!["cache:npm"],
                    ..Default::default()
                },
            ),
            job(
                "build:static",
                "build",
                JobOpts {
                    start: Some(310),
                    end: Some(400),
                    deps: vec!["build:server"],
                    ..Default::default()
                },
            ),
            job(
                "e2e: [1]",
                "test",
                JobOpts {
                    start: Some(410),
                    end: Some(500),
                    deps: vec!["build:static"],
                    ..Default::default()
                },
            ),
            job(
                "e2e: [2]",
                "test",
                JobOpts {
                    start: Some(411),
                    end: Some(510),
                    deps: vec!["build:server"],
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    assert_eq!(
        names(find(&tree, "build")),
        ["build:server", "build:static", "lint"]
    );
    assert_eq!(
        find(&tree, "build:static").after.as_deref(),
        Some("gid://gitlab/Ci::Build/build:server")
    );
    assert_eq!(find(&tree, "lint").after, None);
    assert_eq!(
        find(&tree, "e2e").deps,
        [
            "gid://gitlab/Ci::Build/build:static",
            "gid://gitlab/Ci::Build/build:server"
        ]
    );
}

fn order(stage: &Span) -> Vec<(&str, Option<&str>)> {
    stage
        .children
        .iter()
        .map(|s| (s.name.as_str(), s.after.as_deref()))
        .collect()
}

// эталон для трёх тестов ниже — вывод src/model.mjs на тех же фикстурах

#[test]
fn цикл_зависимостей_не_теряет_узлы_и_каждый_стоит_после_того_кого_ждал() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "a",
                "build",
                JobOpts {
                    start: Some(0),
                    end: Some(10),
                    deps: vec!["b"],
                    ..Default::default()
                },
            ),
            job(
                "b",
                "build",
                JobOpts {
                    start: Some(5),
                    end: Some(20),
                    deps: vec!["a"],
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    assert_eq!(
        order(find(&tree, "build")),
        [
            ("a", Some("gid://gitlab/Ci::Build/b")),
            ("b", Some("gid://gitlab/Ci::Build/a"))
        ]
    );
}

#[test]
fn при_равном_конце_ждавших_джоба_встаёт_за_раньше_стартовавшим_соседом() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "a",
                "build",
                JobOpts {
                    start: Some(0),
                    end: Some(10),
                    ..Default::default()
                },
            ),
            job(
                "b",
                "build",
                JobOpts {
                    start: Some(1),
                    end: Some(10),
                    ..Default::default()
                },
            ),
            job(
                "c",
                "build",
                JobOpts {
                    start: Some(11),
                    end: Some(20),
                    deps: vec!["b", "a"],
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    assert_eq!(
        order(find(&tree, "build")),
        [
            ("a", None),
            ("c", Some("gid://gitlab/Ci::Build/a")),
            ("b", None)
        ]
    );
}

#[test]
fn джоба_без_старта_стоит_в_стейдже_последней() {
    let tree = build(&raw_pipeline(
        vec![
            job(
                "deploy",
                "build",
                JobOpts {
                    status: "MANUAL",
                    ..Default::default()
                },
            ),
            job(
                "a",
                "build",
                JobOpts {
                    start: Some(0),
                    end: Some(10),
                    ..Default::default()
                },
            ),
        ],
        PipelineOpts::default(),
    ));
    assert_eq!(order(find(&tree, "build")), [("a", None), ("deploy", None)]);
}

#[test]
fn shard_group_name_снимает_оба_формата_суффикса() {
    assert_eq!(
        shard_group_name("tests:e2e:master-stage: [9]"),
        "tests:e2e:master-stage"
    );
    assert_eq!(shard_group_name("rspec 3/10"), "rspec");
    assert_eq!(shard_group_name("build:server"), "build:server");
}

fn шард(name: &str, group: Option<&str>, start: i64, end: i64) -> RawJob {
    RawJob {
        shard_group: group.map(Into::into),
        ..job(
            name,
            "test",
            JobOpts {
                start: Some(start),
                end: Some(end),
                ..Default::default()
            },
        )
    }
}

#[test]
fn шарды_с_явным_ключом_группируются_по_нему_имя_шарда_остаётся() {
    let tree = build(&raw_pipeline(
        vec![
            шард("Stats (webpack)", Some("stats"), 0, 50),
            шард("Stats (turbopack)", Some("stats"), 5, 80),
            шард("solo", None, 1, 2),
        ],
        PipelineOpts::default(),
    ));
    let stage = find(&tree, "test");
    let kinds: Vec<_> = stage
        .children
        .iter()
        .map(|s| (s.kind, s.name.as_str()))
        .collect();
    assert_eq!(kinds, [(Kind::Group, "stats"), (Kind::Job, "solo")]);
    assert_eq!(
        names(&stage.children[0]),
        ["Stats (webpack)", "Stats (turbopack)"]
    );
}

#[test]
fn две_группы_в_стейдже_остаются_группами() {
    let tree = build(&raw_pipeline(
        vec![
            шард("a 1/2", None, 0, 10),
            шард("a 2/2", None, 1, 11),
            шард("b 1/2", None, 2, 12),
            шард("b 2/2", None, 3, 13),
        ],
        PipelineOpts::default(),
    ));
    let kinds: Vec<_> = find(&tree, "test")
        .children
        .iter()
        .map(|s| s.kind)
        .collect();
    assert_eq!(kinds, [Kind::Group, Kind::Group]);
}
