import type { TipDef } from './pick'

/** Каталог подсказок (спека онбординга, § 2). Порядок — приоритет; тексты — `onboarding.<id>.title|text`. */
export const TIPS = [
  { id: 'add-project.input', kind: 'event' },
  { id: 'token.create', kind: 'event' },
  { id: 'header.link', kind: 'event' },
  { id: 'report.detail', kind: 'event' },
  { id: 'project.screen', kind: 'enter' },
  { id: 'project.workflow', kind: 'enter', after: 'project.screen' },
  { id: 'report.hotspots', kind: 'enter' },
  { id: 'report.critical', kind: 'enter', after: 'report.hotspots' },
  { id: 'report.keys', kind: 'enter', after: 'report.critical' },
  { id: 'report.retries', kind: 'enter' },
  { id: 'report.aggregate', kind: 'enter' },
] as const satisfies readonly TipDef[]

export type TipId = (typeof TIPS)[number]['id']
