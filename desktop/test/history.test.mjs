import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp } from 'node:fs/promises'
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
