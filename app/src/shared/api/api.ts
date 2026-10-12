import { DELIVERY } from '../config/delivery'
import * as desktop from './desktop'
import * as selfhosted from './selfhosted'

export type { ApiError, BuildProgress, Result } from './result'

// `DELIVERY` при сборке — литерал: ветка другого варианта вырезается, `@tauri-apps/*` в веб-бандл не попадает
const transport: typeof desktop = DELIVERY === 'desktop' ? desktop : selfhosted

export const {
  addProject, branches, build, clearHistory, findReport, getSettings, history, hosts, hostProvider, login, logout, oauthCancel, oauthLogin, pipelines, projects,
  rebuild, removeHistory, removeProject, removeToken, report, reportFileName, reportTemplate, savedProjects, setCurrentReport, setSettings, setToken, workflows,
} = transport
