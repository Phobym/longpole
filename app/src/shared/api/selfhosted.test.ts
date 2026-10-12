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

test('build: прогресс из потока, отчёт в памяти вкладки, запись в историю браузера', async () => {
  serve(() => [{ progress: { loaded: 1, total: 2 } }, { queued: true }, { progress: { loaded: 2, total: 2 } }, ...built('7', '2026-10-12T10:00:00Z')])
  const progress: unknown[] = []
  const id = value(await api.build(form, (p) => progress.push(p)))
  assert.deepEqual(calls[0], { url: '/api/build', body: { form } })
  assert.deepEqual(progress, [{ loaded: 1, total: 2 }, { loaded: 2, total: 2 }])
  assert.deepEqual(value(await api.report(id)), report('7'))
  assert.equal(value(await api.findReport('gitlab.example.com', request('7'))), id)
  assert.equal(value(await api.findReport('gitlab.example.com', request('8'))), null)
  assert.deepEqual(value(await api.history()), [entry('7', '2026-10-12T10:00:00Z')])
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
