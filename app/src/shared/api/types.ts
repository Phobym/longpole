// ponytail: типы команд ядра, пока их не экспортирует ts-rs (S4: `CmdError`, `Progress`, `HistoryEntry`,
// форма `build`). Когда в schema/ появятся сгенерированные — заменить этот файл реэкспортом.
import type { ErrorCode } from './schema/ErrorCode'

export type ErrorBody = { code: ErrorCode; params: Record<string, string> }

export type CmdError =
  | ({ kind: 'message' } & ErrorBody)
  | { kind: 'fields'; errors: Record<string, ErrorBody> }

export type Progress = { loaded: number; total: number | null }

type AggregateForm = { mode: 'aggregate'; host: string; project: string; ref: string; source: string; last: string; statuses: string[] }

export type BuildForm = { mode: 'link'; url: string } | AggregateForm

export type HistoryEntry = {
  at: string
  host: string
  form: Partial<{ url: string } & AggregateForm> & { mode: BuildForm['mode'] }
  label: { project: string; label: string | null }
}
