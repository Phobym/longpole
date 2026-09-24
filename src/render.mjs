import { readFile } from 'node:fs/promises'

const TEMPLATE_URL = new URL('./template.html', import.meta.url)
const CRITICAL_PATH_URL = new URL('./critical-path.mjs', import.meta.url)

export function toScriptJson(value) {
  return JSON.stringify(value)
    .replace(/</g, '\\u003c')
    .replace(/\u2028/g, '\\u2028')
    .replace(/\u2029/g, '\\u2029')
}

export async function render(report) {
  const [template, criticalPathSource] = await Promise.all([
    readFile(TEMPLATE_URL, 'utf8'),
    readFile(CRITICAL_PATH_URL, 'utf8'),
  ])
  return template
    .replace('/*__CRITICAL_PATH__*/', () => criticalPathSource.replace(/^export /gm, ''))
    .replace('/*__DATA__*/null', () => toScriptJson(report))
}
