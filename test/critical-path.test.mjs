import { test } from 'node:test'
import assert from 'node:assert/strict'
import { criticalPath } from '../src/critical-path.mjs'

const j = (id, start, end, deps = [], kind = 'job', children = []) =>
  ({ id, kind, name: id, start, end, deps, children, attempts: [], queued: null })
const node = (id, kind, children) => ({ id, kind, name: id, start: null, end: null, deps: [], children })
const pipe = (id, stages) => node(id, 'pipeline', stages)

test('цепочка: путь в хронологическом порядке, зазоры только положительные', () => {
  const root = pipe('p', [node('s', 'stage', [j('a', 0, 10), j('b', 15, 20, ['a']), j('c', 20, 30, ['b'])])])
  assert.deepEqual(criticalPath(root), {
    ids: ['a', 'b', 'c'],
    gaps: [{ from: 'a', to: 'b', ms: 5 }],
  })
})

test('ромб: выбирается зависимость, закончившаяся позже', () => {
  const root = pipe('p', [
    node('s1', 'stage', [j('a', 0, 10)]),
    node('s2', 'stage', [j('b', 10, 20, ['a']), j('c', 10, 40, ['a'])]),
    node('s3', 'stage', [j('d', 40, 50, ['b', 'c'])]),
  ])
  assert.deepEqual(criticalPath(root).ids, ['a', 'c', 'd'])
})

test('не запускавшиеся зависимости пропускаются, при равном end берётся первая', () => {
  const root = pipe('p', [node('s', 'stage', [
    j('x', null, null), j('b', 0, 10), j('c', 0, 10), j('d', 10, 20, ['x', 'b', 'c']),
  ])])
  assert.deepEqual(criticalPath(root).ids, ['b', 'd'])
})

test('scope стейджа: путь до его последней джобы', () => {
  const root = pipe('p', [
    node('build', 'stage', [j('a', 0, 10), j('b', 10, 30, ['a'])]),
    node('deploy', 'stage', [j('c', 30, 90, ['b'])]),
  ])
  assert.deepEqual(criticalPath(root, 'build').ids, ['a', 'b'])
})

test('bridge: путь заходит в downstream и возвращается к зависимостям bridge', () => {
  const inner = pipe('p2', [node('t', 'stage', [j('i1', 60, 70), j('i2', 75, 90, ['i1'])])])
  const root = pipe('p', [
    node('build', 'stage', [j('a', 0, 50), j('noise', 0, 5)]),
    node('deploy', 'stage', [j('br', 55, 90, ['a', 'noise'], 'bridge', [inner])]),
  ])
  assert.deepEqual(criticalPath(root), {
    ids: ['a', 'i1', 'i2', 'br'],
    gaps: [{ from: 'a', to: 'i1', ms: 10 }, { from: 'i1', to: 'i2', ms: 5 }],
  })
})

test('scope без запускавшихся джоб даёт пустой путь', () => {
  const root = pipe('p', [node('s', 'stage', [j('m', null, null)])])
  assert.deepEqual(criticalPath(root, 's'), { ids: [], gaps: [] })
})
