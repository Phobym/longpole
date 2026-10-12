import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Report } from '../../../shared/api'
import { renderReport } from './render.ts'

// Шаблон с `$&` в хвосте: подстановка не должна разворачивать шаблоны замены строк
const template = '<head></head><body><script type="application/json" id="data"></script><p>$&</p></body>'

test('данные в теге шаблона, `<` экранирован — как `render` в ядре', () => {
  const report = { mode: 'single', meta: { label: '</script><b>' } } as unknown as Report
  assert.equal(
    renderReport(report, template),
    '<head></head><body><script type="application/json" id="data">{"mode":"single","meta":{"label":"\\u003c/script>\\u003cb>"}}</script><p>$&</p></body>',
  )
})

test('шаблон без места для данных — ошибка, а не файл без отчёта', () => {
  assert.throws(() => renderReport({} as Report, '<body></body>'))
})
