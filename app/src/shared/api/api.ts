import { Channel, invoke } from '@tauri-apps/api/core'
import type { CmdError } from './schema/CmdError'
import type { Form } from './schema/Form'
import type { HistoryEntry } from './schema/HistoryEntry'
import type { Locale } from './schema/Locale'
import type { Page } from './schema/Page'
import type { Pipeline } from './schema/Pipeline'
import type { Progress } from './schema/Progress'
import type { Project } from './schema/Project'

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
    return { ok: false, error: isCmdError(e) ? e : { kind: 'message', code: 'ipc', params: { detail: String(e) } } }
  }
}

export const hosts = () => call<string[]>('hosts')
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
/** Открывает окно отчёта; `onProgress` получает `{ loaded, total }` только от этой сборки. */
export const build = (form: Form, onProgress: (p: Progress) => void) => {
  const channel = new Channel<Progress>(onProgress)
  return call<null>('build', { form, onProgress: channel })
}
export const getLocale = () => call<Locale>('get_locale')
export const setLocale = (locale: Locale) => call<null>('set_locale', { locale })
