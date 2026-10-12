import type { Report } from '../../../shared/api'

/** Пустой тег данных в шаблоне отчёта — как `DATA_PLACEHOLDER` в `core/src/render.rs`. */
const PLACEHOLDER = '<script type="application/json" id="data"></script>'

/**
 * Самодостаточный HTML, как `render` в ядре: отчёт в теге данных собранного шаблона, экранирован только `<`.
 * JSON сериализует браузер, поэтому порядок ключей и запись чисел (`1.0` → `1`) могут отличаться от файла Desktop —
 * отчёт при открытии читает те же данные.
 */
export function renderReport(report: Report, template: string): string {
  const at = template.indexOf(PLACEHOLDER)
  if (at < 0) throw new Error('в шаблоне отчёта нет места для данных')
  const json = JSON.stringify(report).replaceAll('<', '\\u003c')
  const open = PLACEHOLDER.slice(0, -'</script>'.length)
  return `${template.slice(0, at)}${open}${json}</script>${template.slice(at + PLACEHOLDER.length)}`
}
