import type { ApiError, Result } from './api'
import type { Field } from './schema/Field'

/** Исключение для TanStack Query: `queryFn` и `mutationFn` обязаны бросать, чтобы запрос попал в `error`. */
export class ApiFailure extends Error {
  constructor(readonly error: ApiError) {
    super(error.kind === 'message' ? error.code : 'fields')
  }
}

export async function unwrap<T>(result: Promise<Result<T>>): Promise<T> {
  const r = await result
  if (!r.ok) throw new ApiFailure(r.error)
  return r.value
}

/** Ошибка запроса или мутации Query; чужое исключение (не `ApiFailure`) тоже не теряется, а становится `ipc`. */
export const apiError = (e: unknown): ApiError | null =>
  e == null ? null : e instanceof ApiFailure ? e.error : { kind: 'message', code: 'ipc', params: { detail: String(e) } }

/** Ошибка поля формы сборки (их возвращает только `build`). */
export const fieldError = (e: ApiError | null, field: Field) => (e?.kind === 'fields' ? e.errors[field] : undefined)

/** Ошибка без привязки к полю. */
export const messageError = (e: ApiError | null) => (e?.kind === 'message' ? e : undefined)
