import assert from 'node:assert/strict'
import { test } from 'node:test'
import { pickTip, type TipDef } from './pick.ts'

const ORDER: TipDef[] = [
  { id: 'event', kind: 'event' },
  { id: 'first', kind: 'enter' },
  { id: 'second', kind: 'enter' },
  { id: 'chained', kind: 'enter', after: 'first' },
]
const base = { order: ORDER, ready: [] as string[], seen: [] as string[], enabled: true, enterShown: false, active: null as string | null }

test('две enter готовы — выбрана первая по реестру', () => {
  assert.equal(pickTip({ ...base, ready: ['second', 'first'] }), 'first')
})

test('enter уже была за визит — следующая enter не выбирается', () => {
  assert.equal(pickTip({ ...base, ready: ['second'], enterShown: true }), null)
})

test('event выбирается после enter за визит и раньше готовой enter', () => {
  assert.equal(pickTip({ ...base, ready: ['event'], enterShown: true }), 'event')
  assert.equal(pickTip({ ...base, ready: ['first', 'event'] }), 'event')
})

test('открытая подсказка блокирует выбор', () => {
  assert.equal(pickTip({ ...base, ready: ['first'], active: 'event' }), null)
})

test('after не просмотрена — подсказка ждёт', () => {
  assert.equal(pickTip({ ...base, ready: ['chained'] }), null)
  assert.equal(pickTip({ ...base, ready: ['chained'], seen: ['first'] }), 'chained')
})

test('просмотренная не выбирается', () => {
  assert.equal(pickTip({ ...base, ready: ['first'], seen: ['first'] }), null)
})

test('подсказки выключены — ничего', () => {
  assert.equal(pickTip({ ...base, ready: ['event', 'first'], enabled: false }), null)
})
