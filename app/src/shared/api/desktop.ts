/** Транспорт Desktop: команды Tauri через `invoke`. */
import { Channel, invoke } from '@tauri-apps/api/core'
import { transportError, type Result } from './result'
import type { AppSettings } from './schema/AppSettings'
import type { CmdError } from './schema/CmdError'
import type { Form } from './schema/Form'
import type { HistoryEntry } from './schema/HistoryEntry'
import type { HostInfo } from './schema/HostInfo'
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
import type { UserCode } from './schema/UserCode'
import type { Workflow } from './schema/Workflow'

const isCmdError = (e: unknown): e is CmdError =>
  typeof e === 'object' &&
  e !== null &&
  (('kind' in e && e.kind === 'message' && 'code' in e && typeof e.code === 'string') ||
    ('kind' in e && e.kind === 'fields' && 'errors' in e))

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<Result<T>> {
  try {
    return { ok: true, value: await invoke<T>(cmd, args) }
  } catch (e) {
    // отказ самого `invoke` (нет команды, запрет ACL, нет моста); детали `set_token` не печатаем — в них токен
    return { ok: false, error: isCmdError(e) ? e : transportError(cmd === 'set_token' ? cmd : String(e)) }
  }
}

export const hosts = () => call<HostInfo[]>('hosts')
export const setToken = (host: string, token: string) => call<null>('set_token', { host, token })
export const removeToken = (host: string) => call<null>('remove_token', { host })
/** Вход через OAuth; `false` — отменён. GitHub присылает в `onCode` код для ввода в браузере. */
export const oauthLogin = (host: string, clientId: string | null, onCode: (code: UserCode) => void) => {
  const channel = new Channel<UserCode>(onCode)
  return call<boolean>('oauth_login', { host, clientId, onCode: channel })
}
export const oauthCancel = (host: string) => call<null>('oauth_cancel', { host })
export const history = () => call<HistoryEntry[]>('history')
export const removeHistory = (at: string) => call<HistoryEntry[]>('remove_history', { at })
export const clearHistory = () => call<null>('clear_history')
export const projects = (host: string, search: string, after: string | null) =>
  call<Page<Project>>('projects', { host, search, after })
export const branches = (host: string, project: string, search: string) =>
  call<string[]>('branches', { host, project, search })
export const pipelines = (host: string, project: string, ref: string | null, workflow: string | null, after: string | null) =>
  call<Page<Pipeline>>('pipelines', { host, project, ref, workflow, after })
export const workflows = (host: string, project: string) => call<Workflow[]>('workflows', { host, project })
export const hostProvider = (host: string) => call<Provider>('host_provider', { host })
/** Собирает отчёт и возвращает его id для `/report/$id`; `onProgress` получает `{ loaded, total }` только от этой сборки. */
export const build = (form: Form, onProgress: (p: Progress) => void) => {
  const channel = new Channel<Progress>(onProgress)
  return call<number>('build', { form, onProgress: channel })
}
/** Ядро отдаёт JSON текстом: `Report` не клонируется в Rust, а строка уже есть у `render`. */
export const report = async (id: number): Promise<Result<Report>> => {
  const r = await call<string>('report', { id })
  return r.ok ? { ok: true, value: JSON.parse(r.value) as Report } : r
}
export const findReport = (host: string, request: Request) => call<number | null>('find_report', { host, request })
let currentReportChain: Promise<unknown> = Promise.resolve()
/** Вызовы идут строго по порядку: размонтирование (null) не должно обогнать монтирование нового id. */
export const setCurrentReport = (id: number | null) => {
  const next = currentReportChain.then(() => call<null>('set_current_report', { id }))
  currentReportChain = next.catch(() => undefined)
  return next
}
export const savedProjects = () => call<SavedProject[]>('saved_projects')
export const addProject = (input: string) => call<SavedProject>('add_project', { input })
export const removeProject = (project: ProjectRef) => call<SavedProject[]>('remove_project', { project })
export const getSettings = () => call<AppSettings>('get_settings')
export const setSettings = (patch: SettingsPatch) => call<AppSettings>('set_settings', { patch })

// Только Selfhosted: в Desktop вход — `oauthLogin`, выход — `removeToken`. Интерфейс прячет их по `capabilities`.
export const login = (host: string, _returnTo?: string): void => {
  throw new Error(`login ${host}: только Selfhosted`)
}
export const logout = async (host: string): Promise<Result<null>> => ({ ok: false, error: transportError(`logout ${host}: только Selfhosted`) })
