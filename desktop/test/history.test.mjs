import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { createHistory } from '../app/history.mjs'

const tmpFile = async () => join(await mkdtemp(join(tmpdir(), 'pt-history-')), 'history.json')
const entry = (i) => ({ host: 'h', form: { mode: 'link', url: `u${i}` }, request: { mode: 'pipeline', project: 'g/p', pipelineId: String(i) }, label: `#${i}` })

test('новые сверху, лимит, повторный запрос поднимается без дубля', async () => {
  let t = 0
  const history = createHistory({ file: await tmpFile(), limit: 3, now: () => new Date(Date.UTC(2026, 8, 24, 0, t++)) })
  assert.deepEqual(await history.list(), [])
  for (const i of [1, 2, 3, 4]) await history.add(entry(i))
  assert.deepEqual((await history.list()).map((e) => e.label), ['#4', '#3', '#2'])
  await history.add(entry(2))
  const list = await history.list()
  assert.deepEqual(list.map((e) => e.label), ['#2', '#4', '#3'])
  assert.equal(list[0].at, '2026-09-24T00:04:00.000Z')
})

test('remove убирает запись по at, clear очищает список', async () => {
  const file = await tmpFile()
  // разные миллисекунды: иначе add() рискует дать двум записям одинаковый at на быстрой машине
  let t = 0
  const history = createHistory({ file, limit: 3, now: () => new Date(Date.UTC(2026, 8, 24, 0, 0, 0, t++)) })
  await history.add(entry(1))
  await history.add(entry(2))
  const [second, first] = await history.list()
  const afterRemove = await history.remove(first.at)
  assert.deepEqual(afterRemove.map((e) => e.label), ['#2'])
  assert.deepEqual((await history.list()).map((e) => e.label), ['#2'])

  const afterClear = await history.clear()
  assert.deepEqual(afterClear, [])
  assert.deepEqual(await history.list(), [])
})

test('история содержит только белый список полей формы, без токенов', async () => {
  const file = await tmpFile()
  const history = createHistory({ file, limit: 3, now: () => new Date() })
  const e = entry(1)
  await history.add({ ...e, form: { mode: 'link', url: 'u', token: 'secret', extra: 'field' } })
  const list = await history.list()
  assert.deepEqual(list[0].form, { mode: 'link', url: 'u' })
  const content = await readFile(file, 'utf8')
  assert.ok(!content.includes('secret'))
  assert.ok(!content.includes('extra'))
})
