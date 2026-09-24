import { test } from 'node:test'
import assert from 'node:assert/strict'
import { render, toScriptJson } from '../src/render.mjs'

const report = {
  meta: { mode: 'single', host: 'h', project: 'g/p', label: '#1', statusCounts: null, generatedAt: '2026-09-24T00:00:00Z' },
  trees: [{ id: 'p', kind: 'pipeline', name: '</script><b>x', start: 0, end: 1, children: [], deps: [], attempts: [] }],
  agg: null,
}

test('toScriptJson экранирует закрывающий тег и разделители строк', () => {
  assert.equal(toScriptJson({ a: '</script>\u2028' }), '{"a":"\\u003c/script>\\u2028"}')
})

test('render подставляет данные и критический путь без export', async () => {
  const html = await render(report)
  assert.ok(!html.includes('/*__DATA__*/'))
  assert.ok(!html.includes('/*__CRITICAL_PATH__*/'))
  assert.ok(!html.includes('</script><b>'))
  assert.match(html, /\nfunction criticalPath\(root, scopeId = root\.id\)/)
  assert.ok(!/^export /m.test(html))
  const json = html.match(/const REPORT = (.*);\n/)[1]
  assert.deepEqual(JSON.parse(json), report)
})
