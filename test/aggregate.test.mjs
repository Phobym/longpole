import { test } from 'node:test'
import assert from 'node:assert/strict'
import { aggregate, percentile } from '../src/aggregate.mjs'
import { buildTree } from '../src/model.mjs'
import { job, rawPipeline } from './fixtures.mjs'

const tree = (i, { buildEnd, lint }) => buildTree(rawPipeline([
  job('prepare', 'prepare', { start: 0, end: 10 }),
  job('build', 'build', { start: 10, end: buildEnd, deps: ['prepare'] }),
  ...(lint ? [job('lint', 'build', { start: 10, end: 20, deps: ['prepare'] })] : []),
], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) }), { baseUrl: 'https://h' })

const trees = [
  tree(1, { buildEnd: 100, lint: true }),
  tree(2, { buildEnd: 300, lint: true }),
  tree(3, { buildEnd: 200, lint: false }),
]
const child = (span, key) => span.children.find((c) => `${c.kind}:${c.name}` === key)

test('percentile: nearest-rank', () => {
  assert.equal(percentile([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 50), 5)
  assert.equal(percentile([10, 1, 9, 2, 8, 3, 7, 4, 6, 5], 90), 9)
  assert.equal(percentile([7], 90), 7)
  assert.equal(percentile([], 50), null)
})

test('длина пайплайна и «до конца стейджа» — перцентиль по пайплайнам', () => {
  const agg = aggregate(trees)
  assert.equal(agg.id, 'agg')
  assert.equal(agg.name, '3 пайплайнов')
  assert.deepEqual(agg.stats.end, { p50: 200_000, p90: 300_000 })
  const build = child(agg, 'stage:build')
  assert.equal(build.id, 'agg/stage:build')
  assert.deepEqual(build.stats.end, { p50: 200_000, p90: 300_000 })
  assert.deepEqual(child(build, 'job:build').stats.duration, { p50: 190_000, p90: 290_000 })
})

test('присутствие и доля на критическом пути', () => {
  const build = child(aggregate(trees), 'stage:build')
  const lint = child(build, 'job:lint')
  assert.equal(lint.stats.present, 2)
  assert.equal(lint.stats.total, 3)
  assert.equal(lint.stats.critical, 0)
  assert.equal(child(build, 'job:build').stats.critical, 1)
  assert.deepEqual(lint.stats.samples.map((s) => s.tree), [0, 1])
})

test('полоска агрегата: start и end — p50, дети отсортированы по старту', () => {
  const agg = aggregate(trees)
  assert.deepEqual(agg.children.map((c) => c.name), ['prepare', 'build'])
  const build = child(agg, 'stage:build')
  assert.equal(build.start, 10_000)
  assert.equal(build.end, 200_000)
})

test('ретраи усредняются по запускам', () => {
  const withRetry = buildTree(rawPipeline([
    job('e2e', 'test', { start: 0, end: 5, retried: true, status: 'FAILED' }),
    job('e2e', 'test', { start: 6, end: 10 }),
  ]), { baseUrl: 'https://h' })
  const clean = buildTree(rawPipeline([job('e2e', 'test', { start: 0, end: 10 })]), { baseUrl: 'https://h' })
  const e2e = child(child(aggregate([withRetry, clean]), 'stage:test'), 'job:e2e')
  assert.equal(e2e.stats.retries, 0.5)
})
