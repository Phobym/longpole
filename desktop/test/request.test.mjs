import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseFormRequest } from '../app/request.mjs'

const agg = (over = {}) => ({ mode: 'aggregate', host: 'gitlab.example.com', project: 'g/p', ref: 'master', source: '', last: '20', statuses: ['SUCCESS', 'MANUAL'], ...over })

test('ссылка на пайплайн: хост и проект из ссылки', () => {
  assert.deepEqual(parseFormRequest({ mode: 'link', url: ' https://gitlab.litres.io/platform/react/monorepo/-/pipelines/1025721 ' }), {
    ok: true, host: 'gitlab.litres.io', request: { mode: 'pipeline', project: 'platform/react/monorepo', pipelineId: '1025721' },
  })
})

test('ссылка на MR', () => {
  const r = parseFormRequest({ mode: 'link', url: 'https://h.example/g/p/-/merge_requests/7632' })
  assert.deepEqual(r.request, { mode: 'mr', project: 'g/p', mrIid: '7632' })
})

test('неверная ссылка — ошибка поля url', () => {
  const r = parseFormRequest({ mode: 'link', url: 'https://h.example/g/p/-/jobs/1' })
  assert.equal(r.ok, false)
  assert.match(r.errors.url, /pipelines/)
})

test('агрегат: поля приводятся к запросу buildReport', () => {
  assert.deepEqual(parseFormRequest(agg()), {
    ok: true, host: 'gitlab.example.com',
    request: { mode: 'aggregate', project: 'g/p', ref: 'master', source: null, last: 20, statuses: ['SUCCESS', 'MANUAL'] },
  })
  assert.equal(parseFormRequest(agg({ statuses: ['ANY'] })).request.statuses, null)
})

test('агрегат: ошибки по полям', () => {
  const r = parseFormRequest(agg({ host: '--evil', project: 'nope', last: '0', statuses: [] }))
  assert.equal(r.ok, false)
  assert.deepEqual(Object.keys(r.errors).sort(), ['host', 'last', 'project', 'statuses'])
})

test('агрегат: путь проекта не принимает сегменты из точек', () => {
  assert.equal(parseFormRequest(agg({ project: '../evil' })).ok, false)
  assert.equal(parseFormRequest(agg({ project: 'g/..' })).ok, false)
  assert.equal(parseFormRequest(agg({ project: 'g.x/p-1' })).ok, true)
})
