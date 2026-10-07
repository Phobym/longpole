//! Разбор формы и ссылки: `desktop/request.test` и ссылки из `cli-args.test`.

use std::collections::BTreeMap;

use pipeline_trace_core::error::{Error, ErrorCode, Field};
use pipeline_trace_core::request::{
    Form, FormMode, Parsed, Request, Status, link_provider, parse_form,
};
use pipeline_trace_core::source::Provider;

fn link(url: &str) -> Form {
    Form {
        mode: FormMode::Link,
        url: url.into(),
        ..Default::default()
    }
}

fn aggregate(over: impl FnOnce(&mut Form)) -> Form {
    let mut form = Form {
        mode: FormMode::Aggregate,
        host: "gitlab.example.com".into(),
        project: "g/p".into(),
        r#ref: "master".into(),
        last: "20".into(),
        statuses: vec!["SUCCESS".into(), "MANUAL".into()],
        ..Default::default()
    };
    over(&mut form);
    form
}

fn errors(result: Result<Parsed, BTreeMap<Field, Error>>) -> BTreeMap<Field, Error> {
    result.expect_err("ждали ошибки полей")
}

fn fields(errors: &BTreeMap<Field, Error>) -> Vec<Field> {
    errors.keys().copied().collect()
}

#[test]
fn ссылка_на_пайплайн_даёт_хост_и_проект_из_ссылки() {
    let url = " https://gitlab.litres.io/platform/react/monorepo/-/pipelines/1025721 ";
    assert_eq!(
        parse_form(&link(url)),
        Ok(Parsed {
            host: "gitlab.litres.io".into(),
            request: Request::Pipeline {
                project: "platform/react/monorepo".into(),
                pipeline_id: "1025721".into(),
            },
        })
    );
}

#[test]
fn ссылка_на_mr_хвост_ссылки_игнорируется() {
    let parsed = parse_form(&link(
        "https://h.example/g/p/-/merge_requests/7632/pipelines",
    ));
    assert_eq!(
        parsed,
        Ok(Parsed {
            host: "h.example".into(),
            request: Request::Mr {
                project: "g/p".into(),
                mr_iid: "7632".into(),
            },
        })
    );
}

#[test]
fn неверная_ссылка_это_ошибка_поля_url() {
    for url in [
        "",
        "foo",
        "https://h.example/g/p/-/jobs/1",
        "ftp://h/g/p/-/pipelines/1",
    ] {
        assert_eq!(
            errors(parse_form(&link(url))),
            BTreeMap::from([(Field::Url, Error::new(ErrorCode::InvalidLink))]),
            "{url}"
        );
    }
}

#[test]
fn ссылка_проект_проверяется_как_в_форме_а_id_должен_кончаться_границей() {
    for url in [
        "https://h/a/../b/-/pipelines/1",
        "https://h/a b/c/-/pipelines/1",
        "https://h/g/p/-/pipelines/12abc",
    ] {
        assert_eq!(
            fields(&errors(parse_form(&link(url)))),
            [Field::Url],
            "{url}"
        );
    }
    for url in [
        "https://h/g/p/-/pipelines/12?tab=jobs",
        "https://h/g/p/-/pipelines/12#note",
        "https://h/g/p/-/pipelines/12/",
    ] {
        assert!(parse_form(&link(url)).is_ok(), "{url}");
    }
}

#[test]
fn режим_формы_по_умолчанию_агрегат_как_в_старом_коде() {
    assert_eq!(Form::default().mode, FormMode::Aggregate);
}

#[test]
fn хост_ссылки_проверяется_как_хост_формы() {
    let bad = errors(parse_form(&link("https://us er@h/g/p/-/pipelines/1")));
    assert_eq!(fields(&bad), [Field::Url]);
}

#[test]
fn агрегат_поля_приводятся_к_запросу() {
    assert_eq!(
        parse_form(&aggregate(|_| {})),
        Ok(Parsed {
            host: "gitlab.example.com".into(),
            request: Request::Aggregate {
                project: "g/p".into(),
                r#ref: Some("master".into()),
                source: None,
                last: 20,
                statuses: Some(vec![Status::Success, Status::Manual]),
                workflow: None,
            },
        })
    );
}

#[test]
fn агрегат_пустые_ref_и_source_это_none_а_пробелы_обрезаются() {
    let form = aggregate(|f| {
        f.r#ref = "  ".into();
        f.source = " merge_request_event ".into();
        f.host = " h.example ".into();
        f.last = " 7 ".into();
    });
    assert_eq!(
        parse_form(&form),
        Ok(Parsed {
            host: "h.example".into(),
            request: Request::Aggregate {
                project: "g/p".into(),
                r#ref: None,
                source: Some("merge_request_event".into()),
                last: 7,
                statuses: Some(vec![Status::Success, Status::Manual]),
                workflow: None,
            },
        })
    );
}

#[test]
fn статус_any_снимает_фильтр() {
    let parsed = parse_form(&aggregate(|f| f.statuses = vec!["ANY".into()])).expect("форма верна");
    assert_eq!(
        parsed.request,
        Request::Aggregate {
            project: "g/p".into(),
            r#ref: Some("master".into()),
            source: None,
            last: 20,
            statuses: None,
            workflow: None,
        }
    );
}

#[test]
fn агрегат_ошибки_по_полям_с_кодами_и_параметрами() {
    let form = aggregate(|f| {
        f.host = "--evil".into();
        f.project = "nope".into();
        f.last = "0".into();
        f.statuses = vec![];
    });
    assert_eq!(
        errors(parse_form(&form)),
        BTreeMap::from([
            (Field::Host, Error::new(ErrorCode::InvalidHost)),
            (Field::Project, Error::new(ErrorCode::InvalidProject)),
            (
                Field::Last,
                Error::new(ErrorCode::InvalidLast).with("max", 500)
            ),
            (Field::Statuses, Error::new(ErrorCode::InvalidStatuses)),
        ])
    );
}

#[test]
fn last_только_целое_от_1_до_500() {
    for last in ["", "0", "501", "-1", "2.5", "abc"] {
        let errs = errors(parse_form(&aggregate(|f| f.last = last.into())));
        assert_eq!(fields(&errs), [Field::Last], "{last:?}");
    }
    assert!(parse_form(&aggregate(|f| f.last = "500".into())).is_ok());
}

#[test]
fn неизвестный_статус_это_ошибка_statuses() {
    let form = aggregate(|f| f.statuses = vec!["SUCCESS".into(), "WAT".into()]);
    assert_eq!(fields(&errors(parse_form(&form))), [Field::Statuses]);
}

#[test]
fn путь_проекта_не_принимает_сегменты_из_точек() {
    for project in ["../evil", "g/..", "g/./p", "g"] {
        assert!(
            parse_form(&aggregate(|f| f.project = project.into())).is_err(),
            "{project}"
        );
    }
    assert!(parse_form(&aggregate(|f| f.project = "g.x/p-1".into())).is_ok());
}

#[test]
fn хост_принимает_порт_и_отвергает_схему_пробелы_и_флаги() {
    assert!(parse_form(&aggregate(|f| f.host = "example.com:8080".into())).is_ok());
    for host in ["", "http://example.com", "bad host!", "-x"] {
        let errs = errors(parse_form(&aggregate(|f| f.host = host.into())));
        assert_eq!(fields(&errs), [Field::Host], "{host:?}");
    }
}

#[test]
fn ссылки_github_на_run_job_attempt_и_pr() {
    let run = |url: &str| parse_form(&link(url)).map(|p| (p.host, p.request));
    let pipeline = |id: &str| Request::Pipeline {
        project: "o/r".into(),
        pipeline_id: id.into(),
    };
    for url in [
        "https://github.com/o/r/actions/runs/7",
        "https://github.com/o/r/actions/runs/7/job/99",
        "https://github.com/o/r/actions/runs/7/attempts/2",
        " https://github.com/o/r/actions/runs/7?pr=3 ",
    ] {
        assert_eq!(run(url), Ok(("github.com".into(), pipeline("7"))), "{url}");
    }
    assert_eq!(
        run("https://ghe.example/o/r/pull/42/files"),
        Ok((
            "ghe.example".into(),
            Request::Mr {
                project: "o/r".into(),
                mr_iid: "42".into(),
            }
        ))
    );
}

#[test]
fn тип_хоста_по_форме_ссылки() {
    assert_eq!(
        link_provider("https://ghe.example/o/r/actions/runs/7"),
        Some(Provider::Github)
    );
    assert_eq!(
        link_provider("https://h.example/g/p/-/pipelines/1"),
        Some(Provider::Gitlab)
    );
    assert_eq!(link_provider("https://h.example/g/p"), None);
}

#[test]
fn агрегат_берёт_workflow_из_формы() {
    let parsed = parse_form(&aggregate(|f| f.workflow = " ci.yml ".into())).unwrap();
    let Request::Aggregate { workflow, .. } = parsed.request else {
        panic!("ждали агрегат")
    };
    assert_eq!(workflow.as_deref(), Some("ci.yml"));
}

#[test]
fn старая_запись_истории_без_workflow_читается() {
    let request: Request = serde_json::from_value(serde_json::json!({
        "mode": "aggregate", "project": "g/p", "ref": null, "source": null, "last": 5, "statuses": null,
    }))
    .unwrap();
    assert!(matches!(request, Request::Aggregate { workflow: None, .. }));
}
