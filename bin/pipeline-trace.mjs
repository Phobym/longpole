#!/usr/bin/env node
import { execFile } from 'node:child_process'
import { writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseCliArgs } from '../src/cli-args.mjs'
import { createClient, resolveHost, resolveToken } from '../src/gitlab.mjs'
import { buildReport, defaultFileName } from '../src/report.mjs'
import { render } from '../src/render.mjs'

const OPENER = { darwin: 'open', win32: 'explorer' }[process.platform] ?? 'xdg-open'

async function main() {
  const args = parseCliArgs(process.argv.slice(2))
  if (args.mode === 'help') {
    console.log(args.usage)
    return
  }
  const host = await resolveHost(args.host)
  const gql = createClient({ host, token: await resolveToken(host) })
  const onProgress = ({ loaded, total }) => {
    if (loaded === 0) process.stderr.write(`Загружаю пайплайнов: ${total}\n`)
  }
  const { report, suffix } = await buildReport(args, { gql, host, onProgress })
  const out = resolve(args.out ?? defaultFileName(args.project, suffix))
  await writeFile(out, await render(report))
  console.log(out)
  // браузер не открылся — путь уже напечатан, это не ошибка
  if (args.open) execFile(OPENER, [out], () => {})
}

main().catch((err) => {
  console.error(`pipeline-trace: ${err.message}`)
  process.exitCode = 1
})
