/**
 * Мост Tauri в обычном браузере (`npm run dev` без `tauri dev`): `window.__TAURI_INTERNALS__` с данными в памяти,
 * чтобы форму можно было смотреть и верстать без Rust. Подключается только в dev и только когда настоящего моста нет;
 * в сборку не попадает (`import.meta.env.DEV`).
 */
import { fixtures, type AppSettings, type HistoryEntry, type HostInfo, type Pipeline, type Project, type SavedProject, type SettingsPatch } from '../../shared/api'

const HOST = 'gitlab.example.com'
const now = Date.now()
const iso = (minutesAgo: number) => new Date(now - minutesAgo * 60_000).toISOString()

let settings: AppSettings = { locale: 'ru', theme: 'system', lastProject: { host: HOST, path: 'g/p' } }
const hosts: HostInfo[] = [
  { host: HOST, source: 'keychain' },
  { host: 'gitlab.com', source: 'glab' },
]
let saved: SavedProject[] = [
  { host: HOST, path: 'g/p', name: 'group / project', addedAt: iso(60 * 24 * 3) },
  { host: HOST, path: 'platform/web-app', name: 'platform / web-app', addedAt: iso(60 * 24) },
  { host: 'gitlab.com', path: 'acme/billing-service', name: 'acme / billing-service', addedAt: iso(30) },
]
let history: HistoryEntry[] = [
  {
    at: iso(12),
    host: HOST,
    form: { mode: 'aggregate', host: HOST, url: '', project: 'g/p', ref: 'main', source: '', last: '20', statuses: ['ANY'] },
    request: { mode: 'aggregate', project: 'g/p', ref: 'main', source: null, last: 20, statuses: null },
    label: { project: 'g/p', label: null },
  },
  {
    at: iso(60 * 5),
    host: HOST,
    form: { mode: 'link', host: HOST, url: `https://${HOST}/platform/web-app/-/pipelines/4821`, project: '', ref: '', source: '', last: '', statuses: [] },
    request: { mode: 'pipeline', project: 'platform/web-app', pipelineId: '4821' },
    label: { project: 'platform/web-app', label: '#4821' },
  },
  {
    at: iso(60 * 26),
    host: HOST,
    form: { mode: 'link', host: HOST, url: `https://${HOST}/g/p/-/merge_requests/312`, project: '', ref: '', source: '', last: '', statuses: [] },
    request: { mode: 'mr', project: 'g/p', mrIid: '312' },
    label: { project: 'g/p', label: '!312' },
  },
]
const branches = ['main', 'develop', 'release/2.4', 'feature/report-redesign', 'hotfix/token-refresh']
const pipelines: Pipeline[] = [
  ['4831', 'success', 'fix(web): Esc на отчёте снимает выделение', 'Виталий М.', 14, 1_563_000],
  ['4830', 'failed', 'feat(core): список хостов с источником токена', 'А. Кузнецова', 52, 1_120_000],
  ['4829', 'running', 'refactor(tauri): отчёт в памяти по id', 'Виталий М.', 70, null],
  ['4828', 'manual', 'chore: bump tauri to 2.12', 'renovate', 95, 902_000],
  ['4827', 'success', 'docs: первый запуск — README, отступления спеки', 'Виталий М.', 60 * 3, 1_498_000],
  ['4826', 'canceled', 'feat(web): экран настроек — хосты, тема, язык', 'Д. Орлов', 60 * 5, 311_000],
  ['4825', 'success', 'fix: пустой GITLAB_HOST, мёртвый код report://', 'Виталий М.', 60 * 8, 1_602_000],
  ['4824', 'success', 'feat(web): сайдбар «Мои проекты», шапка', 'А. Кузнецова', 60 * 26, 1_540_000],
].map(([iid, status, title, author, minutesAgo, duration]) => ({
  id: String(iid),
  iid: String(iid),
  status: String(status),
  source: 'push',
  createdAt: iso(Number(minutesAgo)),
  duration: duration as number | null,
  commit: { sha: 'deadbeef', title: String(title) },
  author: String(author),
}))
const projects: Project[] = [
  { fullPath: 'platform/web-app', name: 'web-app', lastActivityAt: iso(20), defaultBranch: 'main' },
  { fullPath: 'platform/api-gateway', name: 'api-gateway', lastActivityAt: iso(60 * 2), defaultBranch: 'main' },
  { fullPath: 'g/p', name: 'project', lastActivityAt: iso(60 * 7), defaultBranch: 'develop' },
]

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

const commands: Record<string, (args: Record<string, unknown>) => Promise<unknown> | unknown> = {
  get_settings: () => settings,
  set_settings: ({ patch }) => (settings = { ...settings, ...(patch as SettingsPatch) }),
  hosts: () => hosts,
  set_token: () => null,
  remove_token: () => null,
  saved_projects: () => saved,
  add_project: async ({ input }) => {
    await delay(400)
    const path = String(input).replace(/^https?:\/\/[^/]+\//, '').replace(/\/-\/.*$/, '').replace(/\.git$/, '')
    const project: SavedProject = { host: HOST, path, name: path.replace('/', ' / '), addedAt: new Date().toISOString() }
    saved = [...saved, project]
    return project
  },
  remove_project: ({ project }) => {
    const ref = project as SavedProject
    saved = saved.filter((p) => !(p.host === ref.host && p.path === ref.path))
    return saved
  },
  history: () => history,
  remove_history: ({ at }) => (history = history.filter((h) => h.at !== at)),
  clear_history: () => void (history = []),
  projects: ({ search }) => ({ items: projects.filter((p) => p.fullPath.includes(String(search ?? ''))), next: null }),
  branches: ({ search }) => branches.filter((b) => b.includes(String(search ?? ''))),
  pipelines: async () => {
    await delay(300)
    return { items: pipelines, next: null }
  },
  build: async () => {
    await delay(900)
    return 1
  },
  report: async () => JSON.stringify((await fixtures.aggregate()).default),
  find_report: () => 1,
  set_current_report: () => null,
  'plugin:dialog|ask': () => true,
  'plugin:dialog|message': () => null,
  'plugin:dialog|open': () => null,
}

const w = window as unknown as { __TAURI_INTERNALS__?: unknown }
if (!w.__TAURI_INTERNALS__) {
  let callbackId = 0
  w.__TAURI_INTERNALS__ = {
    transformCallback: () => ++callbackId,
    unregisterCallback: () => undefined,
    invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
      const handler = commands[cmd]
      if (!handler) throw { kind: 'message', code: 'ipc', params: { detail: `mock: нет команды ${cmd}` } }
      return handler(args)
    },
  }
}
