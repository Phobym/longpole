export {
  branches, build, clearHistory, getLocale, hosts, history, pipelines, projects, removeHistory, removeToken, setLocale, setToken,
  type ApiError, type Result,
} from './api'
export { isAggNode, type ReportNode } from './node'
export { ApiFailure, apiError, fieldError, messageError, unwrap } from './unwrap'
export type { AggNode } from './schema/AggNode'
export type { ErrorBody } from './schema/ErrorBody'
export type { Excess } from './schema/Excess'
export type { Field } from './schema/Field'
export type { Form } from './schema/Form'
export type { HistoryEntry } from './schema/HistoryEntry'
export type { Hotspot } from './schema/Hotspot'
export type { Kind } from './schema/Kind'
export type { Locale } from './schema/Locale'
export type { Page } from './schema/Page'
export type { Pipeline } from './schema/Pipeline'
export type { Progress } from './schema/Progress'
export type { Project } from './schema/Project'
export type { Report } from './schema/Report'
export type { SingleNode } from './schema/SingleNode'
export type { Stability } from './schema/Stability'
export type { Tree } from './schema/Tree'

/** Эталонные отчёты для dev-сервера; грузятся лениво, в сборку отчёта не попадают. */
export const fixtures = {
  single: () => import('./fixtures/single.json'),
  aggregate: () => import('./fixtures/aggregate.json'),
}
