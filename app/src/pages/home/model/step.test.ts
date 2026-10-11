import assert from 'node:assert/strict'
import { test } from 'node:test'
import { homeStep } from './step.ts'

test('ни проектов, ни хостов — приветствие', () => {
  assert.equal(homeStep(0, 0), 'welcome')
})

test('хост есть, проектов нет — выбор проекта', () => {
  assert.equal(homeStep(0, 1), 'pick')
  assert.equal(homeStep(0, 3), 'pick')
})

test('проекты есть — на проект, хосты не важны', () => {
  assert.equal(homeStep(2, 0), 'redirect')
  assert.equal(homeStep(1, 3), 'redirect')
})
