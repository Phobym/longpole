import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildTree, shardGroupName } from '../src/model.mjs'
import { T0, at, job, rawPipeline } from './fixtures.mjs'

const baseUrl = 'https://h'
const find = (span, name) => span.name === name ? span : span.children.map((c) => find(c, name)).find(Boolean)

test('стейджи идут в порядке запуска, границы стейджа — по его джобам', () => {
  const tree = buildTree(rawPipeline([
    job('prepare:checksum', 'prepare', { start: 0, end: 60 }),
    job('cache:npm', 'cache', { start: 5, end: 120 }),
    job('lint', 'build', { start: 125, end: 200, deps: ['cache:npm'] }),
    job('build:server', 'build', { start: 130, end: 300, deps: ['cache:npm'], queued: 2 }),
    job('deploy', 'deploy', { status: 'MANUAL' }),
  ], { status: 'MANUAL' }), { baseUrl })

  assert.equal(tree.kind, 'pipeline')
  assert.equal(tree.name, '#1')
  assert.equal(tree.start, 0)
  assert.equal(tree.end, 300_000)
  assert.equal(tree.url, 'https://h/g/p/-/pipelines/1')
  assert.deepEqual(tree.children.map((s) => s.name), ['prepare', 'cache', 'build', 'deploy'])

  const build = find(tree, 'build')
  assert.equal(build.kind, 'stage')
  assert.equal(build.start, 125_000)
  assert.equal(build.end, 300_000)
  assert.deepEqual(build.children.map((s) => s.name), ['lint', 'build:server'])

  const server = find(tree, 'build:server')
  assert.equal(server.queued, 2000)
  assert.equal(server.stage, 'build')
  assert.equal(server.status, 'success')
  assert.deepEqual(server.deps, ['gid://gitlab/Ci::Build/cache:npm'])
})

test('manual-джоба без старта остаётся в дереве без времени и не влияет на стейдж', () => {
  const tree = buildTree(rawPipeline([
    job('a', 'build', { start: 0, end: 10 }),
    job('deploy', 'build', { status: 'MANUAL' }),
  ]), { baseUrl })
  const deploy = find(tree, 'deploy')
  assert.equal(deploy.start, null)
  assert.equal(deploy.end, null)
  assert.equal(find(tree, 'build').end, 10_000)
})

test('ретраи уходят в attempts, deps указывают на актуальную попытку', () => {
  const tree = buildTree(rawPipeline([
    job('e2e', 'test', { start: 10, end: 20, status: 'FAILED', retried: true }),
    job('e2e', 'test', { start: 30, end: 40 }),
    job('report', 'reports', { start: 41, end: 45, deps: ['e2e', 'missing'] }),
  ]), { baseUrl })
  const e2e = find(tree, 'e2e')
  assert.equal(e2e.start, 30_000)
  assert.deepEqual(e2e.attempts, [
    { start: 10_000, end: 20_000, status: 'failed', url: 'https://h/g/p/-/jobs/e2e' },
  ])
  assert.deepEqual(find(tree, 'report').deps, ['gid://gitlab/Ci::Build/e2e'])
})

test('шарды одного стейджа объединяются в group, одиночная джоба — нет', () => {
  const tree = buildTree(rawPipeline([
    job('e2e: [1]', 'test', { start: 0, end: 50 }),
    job('e2e: [2]', 'test', { start: 5, end: 80 }),
    job('solo', 'test', { start: 1, end: 2 }),
  ]), { baseUrl })
  const stage = find(tree, 'test')
  assert.deepEqual(stage.children.map((s) => [s.kind, s.name]), [['group', 'e2e'], ['job', 'solo']])
  const group = stage.children[0]
  assert.equal(group.start, 0)
  assert.equal(group.end, 80_000)
  assert.deepEqual(group.children.map((s) => s.name), ['e2e: [1]', 'e2e: [2]'])
})

test('shardGroupName снимает оба формата суффикса', () => {
  assert.equal(shardGroupName('tests:e2e:master-stage: [9]'), 'tests:e2e:master-stage')
  assert.equal(shardGroupName('rspec 3/10'), 'rspec')
  assert.equal(shardGroupName('build:server'), 'build:server')
})

test('джоба, отменённая до старта, не получает конец и не растягивает стейдж', () => {
  const canceled = { ...job('late', 'build', { status: 'CANCELED' }), finishedAt: at(500) }
  const tree = buildTree(rawPipeline([job('a', 'build', { start: 0, end: 10 }), canceled]), { baseUrl })
  assert.equal(find(tree, 'late').end, null)
  assert.equal(find(tree, 'build').end, 10_000)
  assert.equal(tree.end, 10_000)
})

test('downstream: вложен в bridge, время отсчитывается от корня, bridge заканчивается вместе с ним', () => {
  const child = rawPipeline([job('inner', 'test', { start: 110, end: 200 })], {
    id: 'gid://gitlab/Ci::Pipeline/2', iid: '7', createdAt: at(100), project: 'other/proj',
  })
  const bridge = job('trigger', 'deploy', {
    kind: 'BRIDGE', start: 100, end: 150,
    downstream: { id: 'gid://gitlab/Ci::Pipeline/2', project: { fullPath: 'other/proj' } },
  })
  const tree = buildTree(rawPipeline([bridge], { downstream: { [bridge.id]: child } }), { baseUrl })
  const span = find(tree, 'trigger')
  assert.equal(span.kind, 'bridge')
  assert.equal(span.end, 200_000)
  const ds = span.children[0]
  assert.equal(ds.kind, 'pipeline')
  assert.equal(ds.start, 100_000)
  assert.equal(ds.url, 'https://h/other/proj/-/pipelines/7')
  assert.equal(find(ds, 'inner').start, 110_000)
  assert.equal(tree.end, 200_000)
})

test('идущая джоба заканчивается в now', () => {
  const tree = buildTree(rawPipeline([job('a', 'build', { start: 10, status: 'RUNNING' })]), {
    baseUrl, now: Date.parse(T0) + 50_000,
  })
  assert.equal(find(tree, 'a').end, 50_000)
})
