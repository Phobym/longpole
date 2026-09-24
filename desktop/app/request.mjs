import { parseCliArgs } from './core/cli-args.mjs'

const HOST_RE = /^[a-z0-9]([a-z0-9.-]*[a-z0-9])?(?::\d+)?$/i
// сегмент из одних точек (`..`) — не путь проекта
const SEGMENT = '(?!\\.+(?:/|$))[\\w.-]+'
const PROJECT_RE = new RegExp(`^${SEGMENT}(/${SEGMENT})+$`)
const STATUSES = new Set(['SUCCESS', 'MANUAL', 'FAILED', 'CANCELED', 'RUNNING'])
const MAX_LAST = 500

const text = (v) => String(v ?? '').trim()

function parseLink(form) {
  try {
    const args = parseCliArgs([text(form.url)])
    const request = args.mode === 'pipeline'
      ? { mode: 'pipeline', project: args.project, pipelineId: args.pipelineId }
      : { mode: 'mr', project: args.project, mrIid: args.mrIid }
    return { ok: true, host: args.host, request }
  } catch {
    return { ok: false, errors: { url: 'Нужна ссылка вида https://<хост>/<группа>/<проект>/-/pipelines/<id> или …/-/merge_requests/<iid>' } }
  }
}

function parseAggregate(form) {
  const errors = {}
  const host = text(form.host)
  const project = text(form.project)
  const last = Number(text(form.last))
  const statuses = Array.isArray(form.statuses) ? form.statuses : []
  if (!HOST_RE.test(host)) errors.host = 'Укажи хост GitLab, например gitlab.example.com'
  if (!PROJECT_RE.test(project)) errors.project = 'Укажи путь проекта, например group/project'
  if (!Number.isInteger(last) || last < 1 || last > MAX_LAST) errors.last = `Число пайплайнов — от 1 до ${MAX_LAST}`
  const any = statuses.includes('ANY')
  if (!any && (statuses.length === 0 || !statuses.every((s) => STATUSES.has(s)))) errors.statuses = 'Выбери хотя бы один статус или «любой»'
  if (Object.keys(errors).length) return { ok: false, errors }
  return {
    ok: true,
    host,
    request: { mode: 'aggregate', project, ref: text(form.ref) || null, source: text(form.source) || null, last, statuses: any ? null : statuses },
  }
}

export function parseFormRequest(form) {
  return form.mode === 'link' ? parseLink(form) : parseAggregate(form)
}
