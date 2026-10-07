//! Подстановка отчёта в шаблон: `test/render.test.mjs` без проверки критического пути (его считает Rust).
mod fixtures;

use fixtures::{T0, clean_pipeline};
use pipeline_trace_core::model::build_tree;
use pipeline_trace_core::render::{DATA_PLACEHOLDER, render};
use pipeline_trace_core::schema::{Locale, Meta, Report};
use pipeline_trace_core::source::Provider;

const TEMPLATE_HEAD: &str = "<!doctype html><title>t</title><body>";
const TEMPLATE_TAIL: &str = "<script>console.log(1)</script></body>";

fn template() -> String {
    format!("{TEMPLATE_HEAD}{DATA_PLACEHOLDER}{TEMPLATE_TAIL}")
}

fn report(project: &str) -> Report {
    let meta = Meta {
        host: "h".into(),
        project: project.into(),
        label: Some("#1".into()),
        locale: Locale::Ru,
        status_counts: None,
        generated_at: "2026-09-24T00:00:00.000Z".into(),
        provider: Provider::Gitlab,
        needs_missing: false,
    };
    Report::single(meta, &build_tree(&clean_pipeline(1), "https://h", T0))
}

/// Что лежит в `<script type="application/json" id="data">`.
fn embedded(html: &str) -> &str {
    let open = r#"<script type="application/json" id="data">"#;
    let start = html.find(open).expect("тег данных") + open.len();
    let end = start + html[start..].find("</script>").expect("закрывающий тег");
    &html[start..end]
}

#[test]
fn данные_встроены_в_шаблон_остальное_не_тронуто() {
    let html = render(&report("g/p"), &template());
    assert!(html.starts_with(TEMPLATE_HEAD));
    assert!(html.ends_with(TEMPLATE_TAIL));
    assert_eq!(html.matches(r#"id="data""#).count(), 1);
}

#[test]
fn round_trip_json_совпадает_с_отчётом() {
    let report = report("g/p");
    let html = render(&report, &template());
    let parsed: serde_json::Value = serde_json::from_str(embedded(&html)).unwrap();
    assert_eq!(parsed, serde_json::to_value(&report).unwrap());
}

#[test]
fn закрывающий_тег_в_данных_экранируется() {
    let report = report("</script><b>x");
    let html = render(&report, &template());
    assert!(!html.contains("</script><b>"));
    assert!(embedded(&html).contains("\x5cu003c/script>"));
    let parsed: serde_json::Value = serde_json::from_str(embedded(&html)).unwrap();
    assert_eq!(parsed["meta"]["project"], "</script><b>x");
}

#[test]
fn спецпоследовательности_замены_в_данных_остаются_буквальными() {
    let report = report("$& $1 $$ $`");
    let html = render(&report, &template());
    let parsed: serde_json::Value = serde_json::from_str(embedded(&html)).unwrap();
    assert_eq!(parsed["meta"]["project"], "$& $1 $$ $`");
}

#[test]
#[should_panic(expected = "в шаблоне нет места для данных")]
fn шаблон_без_места_для_данных_это_ошибка_сборки() {
    render(&report("g/p"), "<html></html>");
}
