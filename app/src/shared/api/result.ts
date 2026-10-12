import type { CmdError } from './schema/CmdError'

export type ApiError = CmdError
export type Result<T> = { ok: true; value: T } | { ok: false; error: ApiError }

/** Отказ транспорта (нет моста, команды, сети или сервера) — не ответ ядра. */
export const transportError = (detail: string): ApiError => ({ kind: 'message', code: 'transport', params: { detail } })
