import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildTree } from '../src/model.mjs'
import { insights, retryLoss, stageExcess, aggInsights, stabilityByName } from '../src/insights.mjs'
import { aggregate } from '../src/aggregate.mjs'
import { job, rawPipeline } from './fixtures.mjs'

export function samplePipeline(i = 1) {
  return rawPipeline([
    job('build:server', 'build', { start: 0, end: 302 }),
    job('build:static', 'build', { start: 10, end: 242 }),
    job('lint', 'build', { start: 0, end: 50, retried: true, status: 'FAILED' }),
    job('lint', 'build', { start: 60, end: 240 }),
    job('e2e: [1]', 'test', { start: 330, end: 780, deps: ['build:server'] }),
    job('e2e: [3]', 'test', { start: 331, end: 590, deps: ['build:server'], retried: true, status: 'FAILED' }),
    job('e2e: [3]', 'test', { start: 600, end: 850, deps: ['build:server'], retried: true, status: 'FAILED' }),
    job('e2e: [3]', 'test', { start: 860, end: 1318, deps: ['build:server'] }),
    job('report', 'report', { start: 1320, end: 1400, deps: ['e2e: [1]', 'e2e: [3]'] }),
  ], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) })
}
const tree = () => buildTree(samplePipeline(), { baseUrl: 'https://h' })
const find = (s, name) => (s.name === name ? s : s.children.map((c) => find(c, name)).find(Boolean))

test('retryLoss: от старта первой попытки до старта успешной', () => {
  const t = tree()
  assert.equal(retryLoss(find(t, 'e2e: [3]')), 529_000)
  assert.equal(retryLoss(find(t, 'lint')), 60_000)
  assert.equal(retryLoss(find(t, 'build:server')), 0)
})

test('stageExcess: узкое место и отрыв от предпоследней джобы', () => {
  const t = tree()
  assert.deepEqual(stageExcess(find(t, 'build')), { id: 'gid://gitlab/Ci::Build/build:server', peersEnd: 242_000, excess: 60_000 })
  assert.equal(stageExcess(find(t, 'report')), null)
})

test('insights: ретраи и превышение только с критического пути, без двойного счёта', () => {
  const t = tree()
  const r = insights(t)
  assert.deepEqual(r.hotspots.map((h) => [h.kind, h.name, h.saving]), [
    ['retry', 'e2e: [3]', 529_000],
    ['excess', 'build:server', 60_000],
  ])
  assert.equal(r.hotspots[0].retries, 2)
  assert.equal(r.saving, 589_000)
  assert.equal(r.totalRetryLoss, 589_000)
  assert.equal(Object.keys(r.retryLoss).length, 2)
  assert.equal(r.stages[find(t, 'test').id].excess, 538_000)
})

const cleanPipeline = (i) => rawPipeline([
  job('build:server', 'build', { start: 0, end: 302 }),
  job('build:static', 'build', { start: 10, end: 242 }),
  job('lint', 'build', { start: 60, end: 240 }),
  job('e2e: [1]', 'test', { start: 330, end: 780, deps: ['build:server'] }),
  job('e2e: [3]', 'test', { start: 860, end: 1318, deps: ['build:server'] }),
  job('report', 'report', { start: 1320, end: 1400, deps: ['e2e: [1]', 'e2e: [3]'] }),
], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) })

const aggOf = () => aggregate([
  buildTree(samplePipeline(1), { baseUrl: 'https://h' }),
  buildTree(cleanPipeline(2), { baseUrl: 'https://h' }),
])

test('агрегат: retried и средние retryLoss в stats', () => {
  const agg = aggOf()
  const shard = find(agg, 'e2e: [3]')
  assert.equal(shard.stats.retried, 1)
  assert.equal(shard.stats.present, 2)
  assert.equal(shard.stats.retryLoss, 264_500)
  assert.equal(find(agg, 'build:server').stats.retried, 0)
})

test('aggInsights: экономия взвешена долей на критическом пути, стабильность по узлам', () => {
  const agg = aggOf()
  const r = aggInsights(agg)
  assert.deepEqual(r.hotspots.map((h) => [h.kind, h.name, h.saving]), [
    ['excess', 'e2e: [3]', 273_500],
    ['retry', 'e2e: [3]', 264_500],
    ['excess', 'build:server', 60_000],
  ])
  assert.deepEqual(r.stability[find(agg, 'e2e: [3]').id], { retried: 1, present: 2 })
  assert.equal(r.totalRetryLoss, 264_500 + 30_000)
})

test('stabilityByName: метки для пайплайна, открытого из агрегата', () => {
  const s = stabilityByName(aggOf())
  assert.deepEqual(s['e2e: [3]'], { retried: 1, present: 2 })
  assert.deepEqual(s['build:server'], { retried: 0, present: 2 })
})
