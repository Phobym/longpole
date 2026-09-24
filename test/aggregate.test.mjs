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
  assert.deepEqual(e2e.stats.samples.map((s) => s.retries), [1, 0])
})

test('агрегат: порядок стейджей — по declared position (минимум среди деревьев), а не по старту', () => {
  const t = (i) => buildTree(rawPipeline([
    job('prepare', 'prepare', { start: 0, end: 5 }),
    job('build', 'build', { start: 10, end: 100 }),
    job('security:code', 'security', { start: 0, end: 20 }),
  ], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i), stages: ['prepare', 'build', 'security'] }), { baseUrl: 'https://h' })
  const agg = aggregate([t(1), t(2)])
  assert.deepEqual(agg.children.map((s) => s.name), ['prepare', 'build', 'security'])
})

test('агрегат: deps указывают на агрегированные узлы, порядок в стейдже по связям', () => {
  const t = (i) => buildTree(rawPipeline([
    job('cache', 'cache', { start: 0, end: 10 }),
    job('server', 'build', { start: 12, end: 50, deps: ['cache'] }),
    job('lint', 'build', { start: 20, end: 100, deps: ['cache'] }),
    job('static', 'build', { start: 51, end: 60, deps: ['server'] }),
  ], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) }), { baseUrl: 'https://h' })
  const agg = aggregate([t(1), t(2)])
  const build = child(agg, 'stage:build')
  assert.deepEqual(build.children.map((c) => c.name), ['server', 'static', 'lint'])
  const staticJob = child(build, 'job:static')
  assert.deepEqual(staticJob.deps, ['agg/stage:build/job:server'])
  assert.equal(staticJob.after, 'agg/stage:build/job:server')
  assert.deepEqual(child(build, 'job:lint').deps, ['agg/stage:cache/job:cache'])
  assert.equal(child(build, 'job:lint').after, null)
  assert.deepEqual(build.deps, [])
})

test('порядок стейджей в агрегате: пайплайн из одного стейджа не сдвигает его вперёд', () => {
  const full = buildTree(rawPipeline([
    job('prep', 'prepare', { start: 0, end: 10 }),
    job('npm', 'cache', { start: 10, end: 20 }),
    job('srv', 'build', { start: 20, end: 30 }),
    job('scan', 'security', { start: 0, end: 5 }),
  ], { id: 'gid://gitlab/Ci::Pipeline/1', iid: '1', stages: ['prepare', 'cache', 'build', 'security'] }), { baseUrl: 'https://h' })
  const publish = buildTree(rawPipeline([job('ver', 'build', { start: 0, end: 60 })], {
    id: 'gid://gitlab/Ci::Pipeline/2', iid: '2', stages: ['build'],
  }), { baseUrl: 'https://h' })
  assert.deepEqual(aggregate([publish, full, publish]).children.map((c) => c.name), ['prepare', 'cache', 'build', 'security'])
})
