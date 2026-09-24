import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildReport, defaultFileName } from '../src/report.mjs'
import { job, rawPipeline } from './fixtures.mjs'

const pipelineResponse = (raw) => ({
  project: { pipeline: { ...raw.pipeline, jobs: { nodes: raw.jobs, pageInfo: { hasNextPage: false, endCursor: null } } } },
})
const sample = (i) => rawPipeline([job('build', 'build', { start: 0, end: 60 })], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) })

function fakeGql({ list = [], headPipeline = null } = {}) {
  const gql = async (query, vars) => {
    if (query.includes('mergeRequest(')) return { project: { mergeRequest: { headPipeline } } }
    if (query.includes('pipelines(')) return { project: { pipelines: { nodes: list, pageInfo: { hasNextPage: false, endCursor: null } } } }
    return pipelineResponse(sample(vars.id.split('/').pop()))
  }
  return Object.assign(gql, { host: 'h.example' })
}
const fixedNow = () => new Date('2026-09-24T10:00:00Z')

test('pipeline: одно дерево, без агрегата, прогресс 0 → 1', async () => {
  const progress = []
  const { report, suffix } = await buildReport(
    { mode: 'pipeline', project: 'g/p', pipelineId: '5' },
    { gql: fakeGql(), host: 'h.example', now: fixedNow, onProgress: (p) => progress.push(p) },
  )
  assert.equal(suffix, '5')
  assert.equal(report.meta.mode, 'single')
  assert.equal(report.meta.label, '#5')
  assert.equal(report.meta.generatedAt, '2026-09-24T10:00:00.000Z')
  assert.equal(report.trees.length, 1)
  assert.equal(report.agg, null)
  assert.equal(report.insights.trees.length, 1)
  assert.equal(report.insights.stability, null)
  assert.deepEqual(progress, [{ loaded: 0, total: 1 }, { loaded: 1, total: 1 }])
})

test('mr: пайплайн берётся из headPipeline, суффикс mr<iid>', async () => {
  const { report, suffix } = await buildReport(
    { mode: 'mr', project: 'g/p', mrIid: '7' },
    { gql: fakeGql({ headPipeline: { id: 'gid://gitlab/Ci::Pipeline/9' } }), host: 'h.example' },
  )
  assert.equal(suffix, 'mr7')
  assert.equal(report.trees[0].name, '#9')
})

test('aggregate: агрегат, стабильность, метка и счётчики статусов', async () => {
  const list = [{ id: 'gid://gitlab/Ci::Pipeline/1', status: 'SUCCESS' }, { id: 'gid://gitlab/Ci::Pipeline/2', status: 'SUCCESS' }]
  const { report, suffix } = await buildReport(
    { mode: 'aggregate', project: 'g/p', ref: 'master', source: null, statuses: ['SUCCESS'], last: 2 },
    { gql: fakeGql({ list }), host: 'h.example' },
  )
  assert.equal(suffix, 'master')
  assert.equal(report.meta.mode, 'aggregate')
  assert.equal(report.meta.label, 'master')
  assert.deepEqual(report.meta.statusCounts, { SUCCESS: 2 })
  assert.equal(report.trees.length, 2)
  assert.ok(report.agg)
  assert.ok(report.insights.agg)
  assert.ok(report.insights.stability)
})

test('aggregate без подходящих пайплайнов — понятная ошибка', async () => {
  await assert.rejects(
    buildReport({ mode: 'aggregate', project: 'g/p', ref: 'x', source: null, statuses: ['SUCCESS'], last: 5 }, { gql: fakeGql(), host: 'h.example' }),
    /ни один пайплайн/,
  )
})

test('defaultFileName', () => {
  assert.equal(defaultFileName('group/proj', 'master'), 'pipeline-trace-group-proj-master.html')
})
