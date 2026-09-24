import { test } from 'node:test'
import assert from 'node:assert/strict'
import { listBranches, listProjects, recentPipelines } from '../src/browse.mjs'

const fake = (handler) => Object.assign(async (q, v) => handler(q, v), { host: 'h.example' })

test('listProjects: поля, курсор, пустой поиск передаётся как null', async () => {
  const seen = []
  const gql = fake((q, v) => {
    seen.push(v)
    return { projects: { pageInfo: { hasNextPage: true, endCursor: 'c1' }, nodes: [
      { fullPath: 'g/p', nameWithNamespace: 'G / P', lastActivityAt: '2026-09-24T10:00:00Z', repository: { rootRef: 'main' } },
      { fullPath: 'g/empty', nameWithNamespace: 'G / Empty', lastActivityAt: '2026-09-23T10:00:00Z', repository: null },
    ] } }
  })
  const r = await listProjects(gql, { search: '  ' })
  assert.deepEqual(r, {
    items: [
      { fullPath: 'g/p', name: 'G / P', lastActivityAt: '2026-09-24T10:00:00Z', defaultBranch: 'main' },
      { fullPath: 'g/empty', name: 'G / Empty', lastActivityAt: '2026-09-23T10:00:00Z', defaultBranch: null },
    ],
    next: 'c1',
  })
  assert.equal(seen[0].search, null)
  await listProjects(gql, { search: 'mono', after: 'c1' })
  assert.deepEqual(seen[1], { search: 'mono', after: 'c1' })
})

test('listBranches: шаблон поиска и основная ветка первой', async () => {
  const seen = []
  const gql = fake((q, v) => {
    seen.push(v)
    return { project: { repository: { rootRef: 'master', branchNames: ['feature/a', 'master', 'fix/b'] } } }
  })
  assert.deepEqual(await listBranches(gql, 'g/p', 'a'), ['master', 'feature/a', 'fix/b'])
  assert.equal(seen[0].pattern, '*a*')
  await listBranches(gql, 'g/p')
  assert.equal(seen[1].pattern, '*')
})

test('recentPipelines: преобразование полей и курсор', async () => {
  const gql = fake(() => ({ project: { pipelines: { pageInfo: { hasNextPage: false, endCursor: null }, nodes: [
    { id: 'gid://gitlab/Ci::Pipeline/1026324', iid: '51256', status: 'MANUAL', source: 'push', createdAt: '2026-09-24T19:02:41+03:00', duration: 3952, commit: { shortId: '718e7e48', title: 'Publish' }, user: { username: 'u' } },
    { id: 'gid://gitlab/Ci::Pipeline/7', iid: '1', status: 'RUNNING', source: 'web', createdAt: '2026-09-24T19:00:00+03:00', duration: null, commit: null, user: null },
  ] } } }))
  assert.deepEqual(await recentPipelines(gql, 'g/p', { ref: 'master' }), {
    items: [
      { id: '1026324', iid: '51256', status: 'manual', source: 'push', createdAt: '2026-09-24T19:02:41+03:00', duration: 3_952_000, commit: { sha: '718e7e48', title: 'Publish' }, author: 'u' },
      { id: '7', iid: '1', status: 'running', source: 'web', createdAt: '2026-09-24T19:00:00+03:00', duration: null, commit: null, author: null },
    ],
    next: null,
  })
})

test('нет проекта — понятная ошибка', async () => {
  const gql = fake(() => ({ project: null }))
  await assert.rejects(listBranches(gql, 'no/such'), /Проект no\/such на h\.example/)
  await assert.rejects(recentPipelines(gql, 'no/such'), /Проект no\/such на h\.example/)
})
