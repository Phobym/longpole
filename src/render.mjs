import { readFile } from 'node:fs/promises'

const TEMPLATE_URL = new URL('./template.html', import.meta.url)
const CRITICAL_PATH_URL = new URL('./critical-path.mjs', import.meta.url)

export function toScriptJson(value) {
  let json = JSON.stringify(value)
  json = json.replace(/</g, '\\u003c')
  // Handle U+2028 and U+2029 which cannot appear in regex literals
  for (let i = 0; i < json.length; i++) {
    if (json.charCodeAt(i) === 0x2028) {
      json = json.slice(0, i) + '\\u2028' + json.slice(i + 1)
      i += 5
    } else if (json.charCodeAt(i) === 0x2029) {
      json = json.slice(0, i) + '\\u2029' + json.slice(i + 1)
      i += 5
    }
  }
  return json
}

export async function render(report) {
  const [template, criticalPathSource] = await Promise.all([
    readFile(TEMPLATE_URL, 'utf8'),
    readFile(CRITICAL_PATH_URL, 'utf8'),
  ])
  // функции-заменители: в данных могут встретиться последовательности вида $&
  return template
    .replace('/*__CRITICAL_PATH__*/', () => criticalPathSource.replace(/^export /gm, ''))
    .replace('/*__DATA__*/null', () => toScriptJson(report))
}
