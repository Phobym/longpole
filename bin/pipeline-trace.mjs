#!/usr/bin/env node
import { execFile } from 'node:child_process'
import { writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseCliArgs } from '../src/cli-args.mjs'
import { createClient, fetchPipeline, listPipelines, mrHeadPipeline, resolveHost, resolveToken } from '../src/gitlab.mjs'
import { buildTree } from '../src/model.mjs'
import { aggregate } from '../src/aggregate.mjs'
import { render } from '../src/render.mjs'

const OPENER = { darwin: 'open', win32: 'explorer' }[process.platform] ?? 'xdg-open'
const slug = (s) => s.replace(/[^\w.-]+/g, '-')

async function main() {
  const args = parseCliArgs(process.argv.slice(2))
  if (args.mode === 'help') {
    console.log(args.usage)
    return
  }
  const host = await resolveHost(args.host)
  const gql = createClient({ host, token: await resolveToken(host) })

  let ids
  let statusCounts = null
  let suffix
  if (args.mode === 'pipeline') {
    ids = [`gid://gitlab/Ci::Pipeline/${args.pipelineId}`]
    suffix = args.pipelineId
  } else if (args.mode === 'mr') {
    ids = [await mrHeadPipeline(gql, args.project, args.mrIid)]
    suffix = `mr${args.mrIid}`
  } else {
    ;({ ids, counts: statusCounts } = await listPipelines(gql, args.project, args))
    if (ids.length === 0) throw new Error('Под фильтры не попал ни один пайплайн: проверь --ref, --source и --status')
    suffix = args.ref ?? args.source ?? 'all'
  }

  process.stderr.write(`Загружаю пайплайнов: ${ids.length}\n`)
  const raws = await Promise.all(ids.map((id) => fetchPipeline(gql, args.project, id)))
  const trees = raws.map((raw) => buildTree(raw, { baseUrl: `https://${host}` }))
  const isAggregate = args.mode === 'aggregate'
  const meta = {
    mode: isAggregate ? 'aggregate' : 'single',
    host,
    project: args.project,
    label: isAggregate ? [args.ref, args.source].filter(Boolean).join(' · ') || 'все пайплайны' : trees[0].name,
    statusCounts,
    generatedAt: new Date().toISOString(),
  }

  const out = resolve(args.out ?? `pipeline-trace-${slug(args.project)}-${slug(suffix)}.html`)
  await writeFile(out, await render({ meta, trees, agg: isAggregate ? aggregate(trees) : null }))
  console.log(out)
  // браузер не открылся — путь уже напечатан, это не ошибка
  if (args.open) execFile(OPENER, [out], () => {})
}

main().catch((err) => {
  console.error(`pipeline-trace: ${err.message}`)
  process.exitCode = 1
})
