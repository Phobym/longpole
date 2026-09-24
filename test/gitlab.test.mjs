import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  createClient, fetchPipeline, listPipelines, mrHeadPipeline, resolveHost, resolveToken,
} from '../src/gitlab.mjs'

const ok = (data) => ({ ok: true, status: 200, json: async () => ({ data }) })
const fakeGql = (handler) => Object.assign(async (query, vars) => handler(query, vars), { host: 'h.example' })
const failingExec = async () => { throw new Error('glab: not found') }

test('createClient шлёт POST с Bearer-токеном и возвращает data', async () => {
  const calls = []
  const gql = createClient({ host: 'h.example', token: 'tok', fetch: async (url, init) => { calls.push([url, init]); return ok({ x: 1 }) } })
  assert.deepEqual(await gql('{ x }', { a: 1 }), { x: 1 })
  assert.equal(gql.host, 'h.example')
  const [url, init] = calls[0]
  assert.equal(url, 'https://h.example/api/graphql')
  assert.equal(init.method, 'POST')
  assert.equal(init.headers.Authorization, 'Bearer tok')
  assert.deepEqual(JSON.parse(init.body), { query: '{ x }', variables: { a: 1 } })
})

test('createClient: 401 и ошибки GraphQL превращаются в понятные ошибки', async () => {
  const unauthorized = createClient({ host: 'h.example', token: 't', fetch: async () => ({ ok: false, status: 401 }) })
  await assert.rejects(unauthorized('{ x }'), /h\.example.*401/)
  const broken = createClient({
    host: 'h.example', token: 't',
    fetch: async () => ({ ok: true, status: 200, json: async () => ({ errors: [{ message: 'Field x missing' }] }) }),
  })
  await assert.rejects(broken('{ x }'), /Field x missing/)
})

test('createClient держит не больше 4 запросов одновременно', async () => {
  let inFlight = 0
  let max = 0
  const gql = createClient({
    host: 'h', token: 't',
    fetch: async () => {
      inFlight++
      max = Math.max(max, inFlight)
      await new Promise((r) => setTimeout(r, 5))
      inFlight--
      return ok({})
    },
  })
  await Promise.all(Array.from({ length: 10 }, () => gql('{ x }')))
  assert.equal(max, 4)
})

const pipelineData = (id, jobs, pageInfo = { hasNextPage: false, endCursor: null }) => ({
  project: { pipeline: { id, iid: '1', status: 'SUCCESS', createdAt: 'T', finishedAt: null, ref: 'master', path: '/p', jobs: { nodes: jobs, pageInfo } } },
})

test('fetchPipeline склеивает страницы джоб', async () => {
  const gql = fakeGql((q, v) => v.after === 'c1'
    ? pipelineData(v.id, [{ id: 'j2', downstreamPipeline: null }])
    : pipelineData(v.id, [{ id: 'j1', downstreamPipeline: null }], { hasNextPage: true, endCursor: 'c1' }))
  const raw = await fetchPipeline(gql, 'g/p', 'P1')
  assert.equal(raw.project, 'g/p')
  assert.equal(raw.pipeline.id, 'P1')
  assert.equal(raw.pipeline.jobs, undefined)
  assert.deepEqual(raw.jobs.map((j) => j.id), ['j1', 'j2'])
  assert.deepEqual(raw.downstream, {})
})

test('fetchPipeline загружает downstream из его проекта', async () => {
  const seen = []
  const gql = fakeGql((q, v) => {
    seen.push([v.project, v.id])
    return v.id === 'P1'
      ? pipelineData('P1', [{ id: 'br', downstreamPipeline: { id: 'P2', project: { fullPath: 'other/proj' } } }])
      : pipelineData('P2', [])
  })
  const raw = await fetchPipeline(gql, 'g/p', 'P1')
  assert.deepEqual(seen, [['g/p', 'P1'], ['other/proj', 'P2']])
  assert.equal(raw.downstream.br.project, 'other/proj')
})

test('fetchPipeline: нет проекта или пайплайна', async () => {
  await assert.rejects(fetchPipeline(fakeGql(() => ({ project: null })), 'g/p', 'P1'), /Проект g\/p на h\.example/)
  await assert.rejects(fetchPipeline(fakeGql(() => ({ project: { pipeline: null } })), 'g/p', 'P1'), /Пайплайн P1/)
})

test('listPipelines фильтрует статусы, считает их и останавливается на last', async () => {
  const pages = {
    null: { nodes: [{ id: 'a', status: 'SUCCESS' }, { id: 'b', status: 'FAILED' }], pageInfo: { hasNextPage: true, endCursor: 'c1' } },
    c1: { nodes: [{ id: 'c', status: 'MANUAL' }, { id: 'd', status: 'SUCCESS' }], pageInfo: { hasNextPage: true, endCursor: 'c2' } },
  }
  const vars = []
  const gql = fakeGql((q, v) => { vars.push(v); return { project: { pipelines: pages[v.after] } } })
  const result = await listPipelines(gql, 'g/p', { ref: 'master', source: null, statuses: ['SUCCESS', 'MANUAL'], last: 2 })
  assert.deepEqual(result, { ids: ['a', 'c'], counts: { SUCCESS: 1, MANUAL: 1 } })
  assert.equal(vars.length, 2)
  assert.equal(vars[0].ref, 'master')
})

test('mrHeadPipeline возвращает gid и падает, если пайплайна нет', async () => {
  const gql = fakeGql(() => ({ project: { mergeRequest: { headPipeline: { id: 'P9' } } } }))
  assert.equal(await mrHeadPipeline(gql, 'g/p', '12'), 'P9')
  await assert.rejects(
    mrHeadPipeline(fakeGql(() => ({ project: { mergeRequest: { headPipeline: null } } })), 'g/p', '12'),
    /MR !12/,
  )
})

test('resolveToken: glab, затем GITLAB_TOKEN с проверкой GITLAB_HOST', async () => {
  assert.equal(await resolveToken('h', { env: {}, exec: async () => ({ stdout: 'from-glab\n' }) }), 'from-glab')
  assert.equal(await resolveToken('h', { env: { GITLAB_TOKEN: 'from-env', GITLAB_HOST: 'https://h/' }, exec: failingExec }), 'from-env')
  await assert.rejects(resolveToken('h', { env: {}, exec: failingExec }), /glab auth login --hostname h/)
  await assert.rejects(resolveToken('unknown.host', { env: { GITLAB_TOKEN: 'from-env' }, exec: failingExec }), /GITLAB_HOST=unknown.host/)
  assert.equal(await resolveToken('h', { env: { GITLAB_TOKEN: 'from-env', GITLAB_HOST: 'h' }, exec: async (cmd, args) => args[2] === 'host' ? { stdout: 'h\n' } : { stdout: '' } }), 'from-env')
})

test('resolveToken: GITLAB_TOKEN для хоста glab по умолчанию без GITLAB_HOST', async () => {
  const exec = async (cmd, args) => ({ stdout: args[2] === 'host' ? 'h\n' : '' })
  assert.equal(await resolveToken('h', { env: { GITLAB_TOKEN: 'from-env' }, exec }), 'from-env')
})

test('resolveHost: явный хост, затем glab, затем ошибка', async () => {
  assert.equal(await resolveHost('x.example', { exec: failingExec }), 'x.example')
  assert.equal(await resolveHost(null, { exec: async () => ({ stdout: 'g.example\n' }) }), 'g.example')
  await assert.rejects(resolveHost(null, { exec: failingExec }), /--host/)
})

test('resolveHost проверяет корректность хоста', async () => {
  await assert.rejects(resolveHost('--evil', { exec: failingExec }), /Некорректный хост/)
  await assert.rejects(resolveHost('http://example.com', { exec: failingExec }), /Некорректный хост/)
  assert.equal(await resolveHost('example.com:8080', { exec: failingExec }), 'example.com:8080')
})

test('fetchPipeline: лимит на количество страниц джоб', async () => {
  let pageCount = 0
  const gql = fakeGql((q, v) => {
    pageCount++
    const hasNextPage = pageCount < 51
    return pipelineData(v.id, [{ id: `j${pageCount}`, downstreamPipeline: null }], { hasNextPage, endCursor: hasNextPage ? `c${pageCount}` : null })
  })
  await assert.rejects(fetchPipeline(gql, 'g/p', 'P1'), /больше 5000 джоб/)
})

test('fetchPipeline: рекурсия downstream ограничена глубиной 3', async () => {
  let requestCount = 0
  const pipelines = {}
  for (let i = 0; i < 5; i++) {
    pipelines[`P${i}`] = {
      id: `P${i}`,
      downstreamPipeline: i < 4 ? { id: `P${i + 1}`, project: { fullPath: 'g/p' } } : null,
    }
  }
  const gql = fakeGql((q, v) => {
    requestCount++
    const p = pipelines[v.id]
    return {
      project: {
        pipeline: {
          id: v.id, iid: '1', status: 'SUCCESS', createdAt: 'T', finishedAt: null, ref: 'master', path: '/p',
          jobs: {
            nodes: p.downstreamPipeline ? [{ id: `br${v.id}`, downstreamPipeline: p.downstreamPipeline }] : [],
            pageInfo: { hasNextPage: false, endCursor: null },
          },
        },
      },
    }
  })
  const raw = await fetchPipeline(gql, 'g/p', 'P0')
  assert.equal(requestCount, 4)
  const p1 = raw.downstream[`brP0`]
  const p2 = p1.downstream[`brP1`]
  const p3 = p2.downstream[`brP2`]
  assert.deepEqual(p3.downstream, {})
})
