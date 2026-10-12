import type { CmdError } from './schema/CmdError'
import type { Progress } from './schema/Progress'

export type ApiError = CmdError
/** Ход сборки: `queued` — сервер Selfhosted ждёт свободный слот (спека поставки, § 11), затем `{ loaded, total }`. */
export type BuildProgress = Progress | 'queued'
export type Result<T> = { ok: true; value: T } | { ok: false; error: ApiError }

/** Отказ транспорта (нет моста, команды, сети или сервера) — не ответ ядра. */
export const transportError = (detail: string): ApiError => ({ kind: 'message', code: 'transport', params: { detail } })
