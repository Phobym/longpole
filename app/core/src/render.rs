//! Готовый HTML отчёта: данные в собранном шаблоне.

use crate::schema::Report;

/// Пустой тег данных в шаблоне отчёта (`vite.report.config.ts` оставляет его как есть).
pub const DATA_PLACEHOLDER: &str = r#"<script type="application/json" id="data"></script>"#;

/// `\u003c` — JSON возвращает его в `<` сам; записано через `\x5c`, чтобы не путать с экранированием в самом Rust.
const ESCAPED_LT: &str = "\x5cu003c";

/// Шаблон с подставленным отчётом. Экранируется только `<`: в `application/json` другого
/// парсер HTML не трогает, а `JSON.parse` на стороне отчёта разворачивает экранированный символ сам.
///
/// Шаблон без `DATA_PLACEHOLDER` — дефект сборки, `build.rs` проверяет артефакт заранее.
pub fn render(report: &Report, template: &str) -> String {
    let json = serde_json::to_string(report)
        .expect("отчёт сериализуется")
        .replace('<', ESCAPED_LT);
    let (head, tail) = template
        .split_once(DATA_PLACEHOLDER)
        .expect("в шаблоне нет места для данных");
    let open = DATA_PLACEHOLDER.trim_end_matches("</script>");
    format!("{head}{open}{json}</script>{tail}")
}
