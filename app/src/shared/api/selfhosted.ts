/**
 * Транспорт Selfhosted: удалённые операции — `POST /api/<команда>` на сервер инстанса,
 * локальные (история, мои проекты, настройки) — хранилище браузера. Сигнатуры те же, что у `desktop.ts`.
 */
import type { AppSettings } from './schema/AppSettings'
import type { Form } from './schema/Form'
import type { HistoryEntry } from './schema/HistoryEntry'
import type { HostInfo } from './schema/HostInfo'
import type { Locale } from './schema/Locale'
import type { Page } from './schema/Page'
import type { Pipeline } from './schema/Pipeline'
import type { Progress } from './schema/Progress'
import type { Project } from './schema/Project'
import type { ProjectRef } from './schema/ProjectRef'
import type { Provider } from './schema/Provider'
import type { Report } from './schema/Report'
import type { Request } from './schema/Request'
import type { SavedProject } from './schema/SavedProject'
import type { SettingsPatch } from './schema/SettingsPatch'
import type { Workflow } from './schema/Workflow'
// с расширением: тест гоняет файл в Node без сборщика
import { transportError, type ApiError, type BuildProgress, type Result } from './result.ts'

const ok = <T>(value: T): Result<T> => ({ ok: true, value })

type Line<T> = { ok: T } | { err: ApiError } | { progress: Progress } | { queued: true }

const SIGNING_IN = 'longpole.signingIn'
/** Сколько после начала входа новый `authRequired` того же хоста считается его неудачей. */
const SIGN_IN_WINDOW_MS = 60_000

/** Метка начатого входа `{ host, at }` — её пишет `login`; испорченная — как её отсутствие. */
const signingIn = () => {
  try {
    return JSON.parse(sessionStorage.getItem(SIGNING_IN) ?? 'null') as { host: string; at: number } | null
  } catch {
    return null
  }
}
/** `at` метки, которую записала эта страница: она уже уходит к провайдеру, вход ещё не случился. */
let leavingAt: number | undefined

/** Вход в хост начат только что: отказ у провайдера или 401 на свежий токен вернули на ту же страницу. */
const justSignedIn = (host: string) => {
  const mark = signingIn()
  return mark?.host === host && Date.now() - mark.at < SIGN_IN_WINDOW_MS
}

/**
 * Провайдер принял вход в хост — операция с ним прошла: метка снимается, следующий `authRequired` снова уводит на тихий вход.
 * `hosts` и `host_provider` вход не подтверждают: сессия в cookie есть и тогда, когда провайдер отвечает на неё 401.
 * Успех запроса, что был в пути, когда эта страница ушла на вход, — тоже: это ответ на старую сессию.
 */
const confirmSignIn = (host: string) => {
  const mark = signingIn()
  if (mark?.host === host && mark.at !== leavingAt) sessionStorage.removeItem(SIGNING_IN)
}
/** Операция от имени пользователя в хосте: её успех подтверждает вход. */
const confirmingSignIn = async <T>(host: string, r: Promise<Result<T>>) => {
  const result = await r
  if (result.ok) confirmSignIn(host)
  return result
}

/**
 * Вход истёк или отозван (спека, § 5): повторный вход у confidential-приложения тихий, возврат — на текущий URL.
 * Сразу после входа в тот же хост не повторяем — иначе цикл; ошибка доходит до экрана с кнопкой «Войти».
 * Возвращает, ушла ли страница на вход.
 */
const signInAgain = (e: ApiError) => {
  const host = e.kind === 'message' && e.code === 'authRequired' ? e.params.host : undefined
  if (!host || justSignedIn(host)) return false
  login(host)
  return true
}

/**
 * Ответ сервера — NDJSON: у `build` сначала `queued` и строки прогресса, у всех последняя строка `ok` или `err`.
 * `Accept-Language` — язык из настроек: на нём ядро пишет `meta.locale` отчёта.
 */
async function post<T>(cmd: string, args: Record<string, unknown>, onProgress?: (p: BuildProgress) => void): Promise<Result<T>> {
  try {
    const headers = { 'Content-Type': 'application/json', 'Accept-Language': settings().locale }
    const res = await fetch(`/api/${cmd}`, { method: 'POST', headers, body: JSON.stringify(args) })
    if (!res.body) return { ok: false, error: transportError(`HTTP ${res.status}`) }
    const reader = res.body.pipeThrough(new TextDecoderStream()).getReader()
    let rest = ''
    for (;;) {
      const { done, value = '' } = await reader.read()
      const lines = (rest + value).split('\n')
      rest = done ? '' : lines.pop()!
      for (const raw of lines.filter(Boolean)) {
        const line = JSON.parse(raw) as Line<T>
        if ('ok' in line) return ok(line.ok)
        if ('err' in line) {
          // страница уходит на вход: ответа нет, иначе экран на миг покажет ошибку («Войди в …»)
          // ponytail: «Назад» со страницы провайдера из bfcache оставит загрузку — до перезагрузки, она покажет экран
          if (signInAgain(line.err)) return new Promise(() => {})
          return { ok: false, error: line.err }
        }
        if ('progress' in line) onProgress?.(line.progress)
        if ('queued' in line) onProgress?.('queued')
      }
      if (done) return { ok: false, error: transportError(`HTTP ${res.status}: нет строки ok или err`) }
    }
  } catch (e) {
    return { ok: false, error: transportError(String(e)) }
  }
}

const read = <T>(key: string, fallback: T): T => {
  const raw = localStorage.getItem(`longpole.${key}`)
  return raw == null ? fallback : (JSON.parse(raw) as T)
}
const write = <T>(key: string, value: T): T => {
  localStorage.setItem(`longpole.${key}`, JSON.stringify(value))
  return value
}
const storedSettings = () => read<Partial<AppSettings>>('settings', {})
const storedProjects = () => read<SavedProject[]>('projects', [])
const storedHistory = () => read<HistoryEntry[]>('history', [])

const LOCALES: Locale[] = ['ru', 'en', 'fr', 'es', 'de', 'it', 'zh', 'ja']
/** Как `browserLocale` из `shared/i18n` (не импортируем: тянет i18next и словари в тест на Node) и `Locale::system` в ядре. */
const systemLocale = (): Locale => LOCALES.find((l) => navigator.language.toLowerCase().startsWith(l)) ?? 'en'

const settings = (): AppSettings => ({ locale: systemLocale(), theme: 'system', lastProject: null, tips: true, seenTips: [], ...storedSettings() })

export const getSettings = async () => ok(settings())
/** Как `Settings::update` в ядре: поле без значения в патче не трогается. */
export const setSettings = async (patch: SettingsPatch) => {
  write('settings', { ...storedSettings(), ...Object.fromEntries(Object.entries(patch).filter(([, v]) => v !== undefined)) })
  return ok(settings())
}

const same = (a: ProjectRef, b: ProjectRef) => a.host === b.host && a.path === b.path

export const savedProjects = async () => ok(storedProjects())
/** Сервер только разбирает ввод и узнаёт имя проекта, список хранит браузер; повтор обновляет имя, как `Projects::add` в ядре. */
export const addProject = async (input: string): Promise<Result<SavedProject>> => {
  const r = await post<SavedProject>('add_project', { input })
  if (!r.ok) return r
  confirmSignIn(r.value.host)
  const list = storedProjects()
  const found = list.find((p) => same(p, r.value))
  write('projects', found ? list.map((p) => (p === found ? { ...p, name: r.value.name } : p)) : [...list, r.value])
  return ok(found ? { ...found, name: r.value.name } : r.value)
}
/** Удалённый проект перестаёт быть последним — как `forget_project` в Desktop. */
export const removeProject = async (project: ProjectRef) => {
  const { lastProject, ...rest } = storedSettings()
  if (lastProject && same(lastProject, project)) write('settings', rest)
  return ok(write('projects', storedProjects().filter((p) => !same(p, project))))
}

const HISTORY_LIMIT = 20 // как `history::LIMIT` в ядре

export const history = async () => ok(storedHistory())
export const removeHistory = async (at: string) => ok(write('history', storedHistory().filter((e) => e.at !== at)))
export const clearHistory = async () => {
  write('history', [])
  return ok(null)
}

// Запросы приходят от одного сериализатора (ядро на сервере), порядок ключей совпадает
const key = (host: string, request: Request) => JSON.stringify([host, request])

/** Отчёты вкладки: серверного id нет, перезагрузка страницы их теряет. */
// ponytail: память вкладки не ограничена; вытеснять старые, если вкладки живут долго
const reports = new Map<number, { key: string; report: Report; form: Form; fileName: string }>()
let lastId = 0

/**
 * Сервер строит отчёт и отдаёт готовую запись истории (`at`, очищенная ссылка, `request`, `label` — от ядра)
 * и имя файла, как у ⌘S в Desktop; отчёт ложится под `id`.
 */
async function buildAs(id: number, form: Form, fresh: boolean, onProgress: (p: BuildProgress) => void): Promise<Result<number>> {
  const r = await post<{ report: Report; entry: HistoryEntry; fileName: string }>('build', fresh ? { form, fresh } : { form }, onProgress)
  if (!r.ok) return r
  const { report, entry, fileName } = r.value
  confirmSignIn(entry.host)
  const k = key(entry.host, entry.request)
  write('history', [entry, ...storedHistory().filter((e) => key(e.host, e.request) !== k)].slice(0, HISTORY_LIMIT))
  reports.set(id, { key: k, report, form, fileName })
  return ok(id)
}

export const build = (form: Form, onProgress: (p: BuildProgress) => void) => buildAs(++lastId, form, false, onProgress)

/** «Перестроить» (спека, § 10): сервер строит заново мимо кэша, отчёт остаётся под тем же id — адрес страницы не меняется. */
export const rebuild = async (id: number, onProgress: (p: BuildProgress) => void): Promise<Result<number>> => {
  const found = reports.get(id)
  return found ? buildAs(id, found.form, true, onProgress) : expired(id)
}
export const report = async (id: number): Promise<Result<Report>> => {
  const found = reports.get(id)
  return found ? ok(found.report) : expired(id)
}
/** Имя файла «Скачать HTML» (спека, § 10) — то же, что предлагает ⌘S в Desktop. */
export const reportFileName = async (id: number): Promise<Result<string>> => {
  const found = reports.get(id)
  return found ? ok(found.fileName) : expired(id)
}
/** Шаблон самодостаточного отчёта для «Скачать HTML»: сборка кладёт его в статику текстом (`vite.config.ts`). */
export const reportTemplate = async (): Promise<Result<string>> => {
  try {
    const res = await fetch('/report-template.txt')
    return res.ok ? ok(await res.text()) : { ok: false, error: transportError(`HTTP ${res.status}`) }
  } catch (e) {
    return { ok: false, error: transportError(String(e)) }
  }
}
const expired = (id: number): Result<never> => ({ ok: false, error: { kind: 'message', code: 'reportExpired', params: { id: String(id) } } })
export const findReport = async (host: string, request: Request) => {
  const k = key(host, request)
  return ok([...reports].reverse().find(([, r]) => r.key === k)?.[0] ?? null)
}

export const hosts = () => post<HostInfo[]>('hosts', {})

/** Вход переходом всей страницы (спека, § 5): сервер уводит к провайдеру и возвращает на `returnTo` — путь этого сайта. */
export const login = (host: string, returnTo = location.pathname + location.search + location.hash): void => {
  leavingAt = Date.now()
  sessionStorage.setItem(SIGNING_IN, JSON.stringify({ host, at: leavingAt }))
  location.assign(`/auth/login?${new URLSearchParams({ host, return_to: returnTo })}`)
}
/** Стирает вход только в этот хост; JSON-заголовок — как у всех POST (сверка CSRF на сервере). */
export const logout = async (host: string): Promise<Result<null>> => {
  try {
    const res = await fetch(`/auth/logout?${new URLSearchParams({ host })}`, { method: 'POST', headers: { 'Content-Type': 'application/json' } })
    return res.ok ? ok(null) : { ok: false, error: transportError(`HTTP ${res.status}`) }
  } catch (e) {
    return { ok: false, error: transportError(String(e)) }
  }
}
export const hostProvider = (host: string) => post<Provider>('host_provider', { host })
export const projects = (host: string, search: string, after: string | null) => confirmingSignIn(host, post<Page<Project>>('projects', { host, search, after }))
export const branches = (host: string, project: string, search: string) => confirmingSignIn(host, post<string[]>('branches', { host, project, search }))
export const pipelines = (host: string, project: string, ref: string | null, workflow: string | null, after: string | null) =>
  confirmingSignIn(host, post<Page<Pipeline>>('pipelines', { host, project, ref, workflow, after }))
export const workflows = (host: string, project: string) => confirmingSignIn(host, post<Workflow[]>('workflows', { host, project }))

// Только Desktop: хосты и вход задаёт инстанс, ⌘S нет. Интерфейс прячет их по `capabilities`, вызов — ошибка.
const desktopOnly = async (cmd: string): Promise<Result<never>> => ({ ok: false, error: transportError(`${cmd}: только Desktop`) })
export const setToken = () => desktopOnly('set_token')
export const removeToken = () => desktopOnly('remove_token')
export const oauthLogin = () => desktopOnly('oauth_login')
export const oauthCancel = () => desktopOnly('oauth_cancel')
// Экран отчёта зовёт его в обоих вариантах; без ⌘S сообщать некому — не ошибка
export const setCurrentReport = async () => ok(null)
