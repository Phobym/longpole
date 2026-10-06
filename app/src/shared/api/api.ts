import { Channel, invoke } from '@tauri-apps/api/core'
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
import type { Report } from './schema/Report'
import type { Request } from './schema/Request'
import type { SavedProject } from './schema/SavedProject'
import type { SettingsPatch } from './schema/SettingsPatch'

/** Отказ самого `invoke` (нет команды, запрет ACL, нет моста) — не ответ ядра, кода в `ErrorCode` у него нет. */
export type IpcError = { kind: 'message'; code: 'ipc'; params: { detail: string } }
export type ApiError = CmdError | IpcError
export type Result<T> = { ok: true; value: T } | { ok: false; error: ApiError }

const isCmdError = (e: unknown): e is CmdError =>
  typeof e === 'object' &&
  e !== null &&
  (('kind' in e && e.kind === 'message' && 'code' in e && typeof e.code === 'string') ||
    ('kind' in e && e.kind === 'fields' && 'errors' in e))

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<Result<T>> {
  try {
    return { ok: true, value: await invoke<T>(cmd, args) }
  } catch (e) {
    return { ok: false, error: isCmdError(e) ? e : { kind: 'message', code: 'ipc', params: { detail: cmd === 'set_token' ? cmd : String(e) } } }
  }
}

export const hosts = () => call<HostInfo[]>('hosts')
export const setToken = (host: string, token: string) => call<null>('set_token', { host, token })
export const removeToken = (host: string) => call<null>('remove_token', { host })
export const history = () => call<HistoryEntry[]>('history')
export const removeHistory = (at: string) => call<HistoryEntry[]>('remove_history', { at })
export const clearHistory = () => call<null>('clear_history')
export const projects = (host: string, search: string, after: string | null) =>
  call<Page<Project>>('projects', { host, search, after })
export const branches = (host: string, project: string, search: string) =>
  call<string[]>('branches', { host, project, search })
export const pipelines = (host: string, project: string, ref: string | null, after: string | null) =>
  call<Page<Pipeline>>('pipelines', { host, project, ref, after })
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
