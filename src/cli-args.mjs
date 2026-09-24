import { parseArgs } from 'node:util'

const USAGE = `Использование:
  pipeline-trace <url пайплайна или MR>
  pipeline-trace --project group/name [--ref master] [--source merge_request_event] [--last 50]
                 [--status success,manual|any] [--host gitlab.example.com]
Общие флаги: [--out файл.html] [--no-open]`

const URL_RE = /^https?:\/\/([^/]+)\/(.+?)\/-\/(pipelines|merge_requests)\/(\d+)/

export function parseCliArgs(argv) {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      project: { type: 'string' },
      ref: { type: 'string' },
      source: { type: 'string' },
      last: { type: 'string', default: '50' },
      status: { type: 'string', default: 'success,manual' },
      host: { type: 'string' },
      out: { type: 'string' },
      'no-open': { type: 'boolean', default: false },
      help: { type: 'boolean', default: false },
    },
  })
  if (values.help) return { mode: 'help', usage: USAGE }
  const common = { out: values.out ?? null, open: !values['no-open'] }

  if (positionals.length > 0) {
    const match = URL_RE.exec(positionals[0])
    if (!match) throw new Error(`Не похоже на ссылку на пайплайн или MR: ${positionals[0]}\n\n${USAGE}`)
    const [, host, project, kind, id] = match
    return kind === 'pipelines'
      ? { mode: 'pipeline', host, project, pipelineId: id, ...common }
      : { mode: 'mr', host, project, mrIid: id, ...common }
  }

  if (!values.project) throw new Error(`Нужна ссылка или --project\n\n${USAGE}`)
  const last = Number(values.last)
  if (!Number.isInteger(last) || last < 1) throw new Error(`--last должно быть целым числом больше 0: ${values.last}`)
  const statuses = values.status === 'any'
    ? null
    : values.status.split(',').map((s) => s.trim().toUpperCase()).filter(Boolean)
  return {
    mode: 'aggregate', host: values.host ?? null, project: values.project,
    ref: values.ref ?? null, source: values.source ?? null, last, statuses, ...common,
  }
}
