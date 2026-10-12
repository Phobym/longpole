import assert from 'node:assert/strict'
import { beforeEach, test } from 'node:test'
import type { Form } from './schema/Form'

// Хранилище браузера: в Node его нет, подменяем до импорта реализации
const store = new Map<string, string>()
Object.defineProperty(globalThis, 'localStorage', {
  value: {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
    removeItem: (k: string) => void store.delete(k),
  },
})
const session = new Map<string, string>()
Object.defineProperty(globalThis, 'sessionStorage', {
  value: { getItem: (k: string) => session.get(k) ?? null, setItem: (k: string, v: string) => void session.set(k, v) },
})
Object.defineProperty(globalThis, 'navigator', { value: { language: 'fr-FR' } })

const api = await import('./selfhosted.ts')

/** Мок сервера: `POST /api/<cmd>` → строки ответа (у `build` их несколько — NDJSON). */
const calls: { url: string; body: unknown }[] = []
const serve = (reply: (cmd: string, body: any) => unknown[]) => {
  globalThis.fetch = (async (url: string, init: RequestInit) => {
    const body = JSON.parse(String(init.body))
    calls.push({ url, body })
    return new Response(reply(url.replace('/api/', ''), body).map((l) => JSON.stringify(l) + '\n').join(''))
  }) as typeof fetch
}

beforeEach(() => {
  store.clear()
  session.clear()
  calls.length = 0
})

const value = <T>(r: { ok: true; value: T } | { ok: false }) => {
  assert.ok(r.ok)
  return r.value
}

test('настройки: по умолчанию язык браузера, патч сохраняется в хранилище', async () => {
  assert.deepEqual(value(await api.getSettings()), { locale: 'fr', theme: 'system', lastProject: null, tips: true, seenTips: [] })
  await api.setSettings({ theme: 'dark', seenTips: ['a'] })
  assert.deepEqual(value(await api.getSettings()), { locale: 'fr', theme: 'dark', lastProject: null, tips: true, seenTips: ['a'] })
  store.clear()
  assert.equal(value(await api.getSettings()).theme, 'system')
})

test('мои проекты: сервер разбирает ввод, список хранится в браузере', async () => {
  const at = '2026-10-12T10:00:00Z'
  serve((_, { input }) => [{ ok: { host: 'gitlab.example.com', path: input, name: `Name ${input}`, addedAt: at } }])
  await api.addProject('g/a')
  await api.addProject('g/b')
  assert.deepEqual(calls[0], { url: '/api/add_project', body: { input: 'g/a' } })
  serve(() => [{ ok: { host: 'gitlab.example.com', path: 'g/a', name: 'Renamed', addedAt: '2026-10-12T11:00:00Z' } }])
  await api.addProject('g/a')
  assert.deepEqual(value(await api.savedProjects()), [
    { host: 'gitlab.example.com', path: 'g/a', name: 'Renamed', addedAt: at },
    { host: 'gitlab.example.com', path: 'g/b', name: 'Name g/b', addedAt: at },
  ])
  assert.deepEqual(value(await api.removeProject({ host: 'gitlab.example.com', path: 'g/a' })).map((p) => p.path), ['g/b'])
  assert.deepEqual(value(await api.savedProjects()).map((p) => p.path), ['g/b'])
})

test('ошибка сервера доходит как есть, сбой сети — transport', async () => {
  serve(() => [{ err: { kind: 'message', code: 'projectInputInvalid', params: {} } }])
  assert.deepEqual(await api.addProject('x'), { ok: false, error: { kind: 'message', code: 'projectInputInvalid', params: {} } })
  assert.deepEqual(value(await api.savedProjects()), [])
  globalThis.fetch = (async () => {
    throw new TypeError('Failed to fetch')
  }) as typeof fetch
  assert.deepEqual(await api.addProject('x'), { ok: false, error: { kind: 'message', code: 'transport', params: { detail: 'TypeError: Failed to fetch' } } })
})

const form: Form = { mode: 'link', host: 'gitlab.example.com', url: 'https://gitlab.example.com/g/p/-/pipelines/7', project: '', ref: '', source: '', last: '', statuses: [], workflow: '' }
const request = (id: string) => ({ mode: 'pipeline', project: 'g/p', pipelineId: id }) as const
const report = (id: string) => ({ mode: 'single', meta: { project: 'g/p', label: `#${id}` }, tree: {} }) as never
const entry = (id: string, at: string) => ({ at, host: 'gitlab.example.com', form, request: request(id), label: { project: 'g/p', label: `#${id}` } })
const built = (id: string, at: string) => [{ ok: { report: report(id), entry: entry(id, at) } }]

test('build: очередь и прогресс из потока, отчёт в памяти вкладки, запись в историю браузера', async () => {
  serve(() => [{ queued: true }, { progress: { loaded: 1, total: 2 } }, { progress: { loaded: 2, total: 2 } }, ...built('7', '2026-10-12T10:00:00Z')])
  const progress: unknown[] = []
  const id = value(await api.build(form, (p) => progress.push(p)))
  assert.deepEqual(calls[0], { url: '/api/build', body: { form } })
  assert.deepEqual(progress, ['queued', { loaded: 1, total: 2 }, { loaded: 2, total: 2 }])
  assert.deepEqual(value(await api.report(id)), report('7'))
  assert.equal(value(await api.findReport('gitlab.example.com', request('7'))), id)
  assert.equal(value(await api.findReport('gitlab.example.com', request('8'))), null)
  assert.deepEqual(value(await api.history()), [entry('7', '2026-10-12T10:00:00Z')])
})

test('rebuild: build с fresh по той же форме, новый отчёт под тем же id', async () => {
  serve(() => built('7', '2026-10-12T10:00:00Z'))
  const id = value(await api.build(form, () => {}))
  serve(() => [{ progress: { loaded: 1, total: 1 } }, { ok: { report: report('7b'), entry: entry('7', '2026-10-12T11:00:00Z') } }])
  const progress: unknown[] = []
  assert.equal(value(await api.rebuild(id, (p) => progress.push(p))), id)
  assert.deepEqual(calls[1], { url: '/api/build', body: { form, fresh: true } })
  assert.deepEqual(progress, [{ loaded: 1, total: 1 }])
  assert.deepEqual(value(await api.report(id)), report('7b'))
  assert.deepEqual(value(await api.history()), [entry('7', '2026-10-12T11:00:00Z')])
  assert.equal((await api.rebuild(999, () => {})).ok, false)
})

test('отчёта нет в памяти вкладки — reportExpired', async () => {
  assert.deepEqual(await api.report(999), { ok: false, error: { kind: 'message', code: 'reportExpired', params: { id: '999' } } })
})

test('история: тот же запрос поднимается наверх, не больше 20, удаление и очистка', async () => {
  for (let i = 0; i < 21; i++) {
    serve(() => built(String(i), `2026-10-12T10:${String(i).padStart(2, '0')}:00Z`))
    await api.build(form, () => {})
  }
  serve(() => built('5', '2026-10-12T11:00:00Z'))
  await api.build(form, () => {})
  const list = value(await api.history())
  assert.equal(list.length, 20)
  assert.deepEqual(list.slice(0, 3).map((e) => e.request.mode === 'pipeline' && e.request.pipelineId), ['5', '20', '19'])
  assert.deepEqual(value(await api.removeHistory('2026-10-12T11:00:00Z')).map((e) => e.at).slice(0, 1), ['2026-10-12T10:20:00Z'])
  await api.clearHistory()
  assert.deepEqual(value(await api.history()), [])
})

test('удалённый проект перестаёт быть последним; поле патча со значением undefined не трогается', async () => {
  const p = { host: 'gitlab.example.com', path: 'g/a' }
  await api.setSettings({ lastProject: p, theme: 'dark' })
  await api.setSettings({ theme: undefined, tips: false })
  assert.equal(value(await api.getSettings()).theme, 'dark')
  await api.removeProject({ host: 'gitlab.example.com', path: 'g/other' })
  assert.deepEqual(value(await api.getSettings()).lastProject, p)
  await api.removeProject(p)
  assert.equal(value(await api.getSettings()).lastProject, null)
})

test('вход — переход всей страницы на /auth/login с возвратом на текущий URL', () => {
  const assigned: string[] = []
  Object.defineProperty(globalThis, 'location', {
    configurable: true,
    value: { pathname: '/', search: '?a=1', hash: '#/project/h/g', assign: (url: string) => assigned.push(url) },
  })
  api.login('gitlab.example.com')
  api.login('ghe.example.com', '/')
  assert.deepEqual(assigned, ['/auth/login?host=gitlab.example.com&return_to=%2F%3Fa%3D1%23%2Fproject%2Fh%2Fg', '/auth/login?host=ghe.example.com&return_to=%2F'])
})

test('authRequired во время работы — тихий вход в этот хост с возвратом на текущий URL', async () => {
  const assigned: string[] = []
  Object.defineProperty(globalThis, 'location', {
    configurable: true,
    value: { pathname: '/project/gitlab.example.com/g/p', search: '?ref=main', hash: '', assign: (url: string) => assigned.push(url) },
  })
  const authRequired = { kind: 'message', code: 'authRequired', params: { host: 'gitlab.example.com' } } as const
  serve(() => [{ progress: { loaded: 0, total: 1 } }, { err: authRequired }])
  assert.deepEqual(await api.build(form, () => {}), { ok: false, error: authRequired })
  serve(() => [{ err: { kind: 'message', code: 'pipelineNotFound', params: {} } }])
  await api.pipelines('gitlab.example.com', 'g/p', null, null, null)
  assert.deepEqual(assigned, ['/auth/login?host=gitlab.example.com&return_to=%2Fproject%2Fgitlab.example.com%2Fg%2Fp%3Fref%3Dmain'])
})

test('язык отчёта — язык из настроек, а не только браузера', async () => {
  await api.setSettings({ locale: 'de' })
  const sent: RequestInit[] = []
  globalThis.fetch = (async (_: string, init: RequestInit) => {
    sent.push(init)
    return new Response(JSON.stringify({ ok: [] }) + '\n')
  }) as typeof fetch
  await api.workflows('ghe.example.com', 'o/r')
  assert.equal(new Headers(sent[0].headers).get('Accept-Language'), 'de')
})

test('выход: POST /auth/logout с JSON-заголовком, отказ сервера — transport', async () => {
  const sent: { url: string; init: RequestInit }[] = []
  let status = 204
  globalThis.fetch = (async (url: string, init: RequestInit) => {
    sent.push({ url, init })
    return new Response(null, { status })
  }) as typeof fetch
  assert.deepEqual(await api.logout('gitlab.example.com'), { ok: true, value: null })
  assert.deepEqual(sent[0], { url: '/auth/logout?host=gitlab.example.com', init: { method: 'POST', headers: { 'Content-Type': 'application/json' } } })
  status = 403
  assert.deepEqual(await api.logout('gitlab.example.com'), { ok: false, error: { kind: 'message', code: 'transport', params: { detail: 'HTTP 403' } } })
})

test('authRequired сразу после входа в этот хост (отказ или 401 на свежий токен) — без второго входа: ошибка доходит до экрана', async () => {
  const assigned: string[] = []
  Object.defineProperty(globalThis, 'location', {
    configurable: true,
    value: { pathname: '/gitlab.example.com/g/p/-/pipelines/7', search: '', hash: '', assign: (url: string) => assigned.push(url) },
  })
  const authRequired = (host: string) => ({ kind: 'message', code: 'authRequired', params: { host } }) as const
  serve(() => [{ err: authRequired('gitlab.example.com') }])
  await api.build(form, () => {})
  assert.deepEqual(assigned, ['/auth/login?host=gitlab.example.com&return_to=%2Fgitlab.example.com%2Fg%2Fp%2F-%2Fpipelines%2F7'])
  // провайдер вернул на ту же ссылку, сервер снова не пустил
  assert.deepEqual(await api.build(form, () => {}), { ok: false, error: authRequired('gitlab.example.com') })
  assert.equal(assigned.length, 1)
  // другой хост — свой тихий вход
  serve(() => [{ err: authRequired('ghe.example.com') }])
  await api.workflows('ghe.example.com', 'o/r')
  assert.equal(assigned.length, 2)
})
