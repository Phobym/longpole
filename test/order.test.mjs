import { test } from 'node:test'
import assert from 'node:assert/strict'
import { orderByDeps } from '../src/model.mjs'

const s = (id, start, end, deps = [], kind = 'job', children = []) => ({ id, kind, name: id, start, end, deps, children })
const ids = (list) => list.map((x) => x.id)

test('зависимая джоба идёт сразу после своей зависимости', () => {
  const out = orderByDeps([s('server', 0, 50), s('lint', 5, 60), s('static', 55, 70, ['server'])])
  assert.deepEqual(ids(out), ['server', 'static', 'lint'])
  assert.deepEqual(out.map((x) => x.after), [null, 'server', null])
})

test('при нескольких зависимостях родитель — закончившаяся последней', () => {
  const out = orderByDeps([s('a', 0, 30), s('b', 0, 10), s('c', 31, 40, ['b', 'a'])])
  assert.deepEqual(ids(out), ['a', 'c', 'b'])
  assert.equal(out[1].after, 'a')
})

test('при равном end родитель — сосед, стартовавший раньше', () => {
  const out = orderByDeps([s('a', 0, 10), s('b', 1, 10), s('c', 11, 20, ['b', 'a'])])
  assert.deepEqual(ids(out), ['a', 'c', 'b'])
})

test('зависимость вне соседей на порядок не влияет', () => {
  const out = orderByDeps([s('x', 10, 20, ['elsewhere']), s('y', 0, 5)])
  assert.deepEqual(ids(out), ['y', 'x'])
  assert.equal(out[1].after, null)
})

test('группа шардов — одна единица: её ставят после зависимости и после неё ставят зависящих от шардов', () => {
  const group = s('g', 60, 90, ['server'], 'group', [s('g1', 60, 80, ['server']), s('g2', 61, 90, ['server'])])
  const out = orderByDeps([s('server', 0, 50), s('lint', 5, 60), group, s('rep', 91, 95, ['g1', 'g2'])])
  assert.deepEqual(ids(out), ['server', 'g', 'rep', 'lint'])
  assert.deepEqual(out.map((x) => x.after), [null, 'server', 'g', null])
})

test('цикл зависимостей не теряет и не дублирует узлы', () => {
  const out = orderByDeps([s('a', 0, 10, ['b']), s('b', 0, 20, ['a'])])
  assert.deepEqual(ids(out).sort(), ['a', 'b'])
})
