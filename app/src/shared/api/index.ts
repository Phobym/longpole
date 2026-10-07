export {
  addProject, branches, build, clearHistory, findReport, getSettings, history, hosts, hostProvider, pipelines, projects, removeHistory,
  removeProject, removeToken, report, savedProjects, setCurrentReport, setSettings, setToken, workflows,
  type ApiError, type Result,
} from './api'
export { isAggNode, type ReportNode } from './node'
export { ApiFailure, apiError, fieldError, messageError, unwrap } from './unwrap'
export type { AppSettings } from './schema/AppSettings'
export type { AggNode } from './schema/AggNode'
export type { ErrorBody } from './schema/ErrorBody'
export type { Excess } from './schema/Excess'
export type { Field } from './schema/Field'
export type { Form } from './schema/Form'
export type { HistoryEntry } from './schema/HistoryEntry'
export type { HostInfo } from './schema/HostInfo'
export type { Hotspot } from './schema/Hotspot'
export type { Kind } from './schema/Kind'
export type { Locale } from './schema/Locale'
export type { Page } from './schema/Page'
export type { Pipeline } from './schema/Pipeline'
export type { Progress } from './schema/Progress'
export type { Project } from './schema/Project'
export type { ProjectRef } from './schema/ProjectRef'
export type { Provider } from './schema/Provider'
export type { Request } from './schema/Request'
export type { SavedProject } from './schema/SavedProject'
export type { SettingsPatch } from './schema/SettingsPatch'
export type { Status } from './schema/Status'
export type { Report } from './schema/Report'
export type { SingleNode } from './schema/SingleNode'
export type { Stability } from './schema/Stability'
export type { Theme } from './schema/Theme'
export type { TokenSource } from './schema/TokenSource'
export type { Tree } from './schema/Tree'
export type { Workflow } from './schema/Workflow'

/** Эталонные отчёты для dev-сервера; грузятся лениво, в сборку отчёта не попадают. */
export const fixtures = {
  single: () => import('./fixtures/single.json'),
  aggregate: () => import('./fixtures/aggregate.json'),
}
