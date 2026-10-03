// Наборы ключей ru.json и en.json должны совпадать (вложенные ключи — через точку).
// Суффиксы склонений (`_one`, `_few`, …) отбрасываются: в русском их четыре, в английском два.
import { readFileSync } from 'node:fs'

const dir = new URL('../src/shared/i18n/locales/', import.meta.url)
const keys = (obj, prefix = '') =>
  Object.entries(obj).flatMap(([k, v]) =>
    v && typeof v === 'object' ? keys(v, `${prefix}${k}.`) : [`${prefix}${k}`.replace(/_(zero|one|two|few|many|other)$/, '')],
  )
const load = (lang) => new Set(keys(JSON.parse(readFileSync(new URL(`${lang}.json`, dir), 'utf8'))))

const ru = load('ru')
const en = load('en')
const onlyRu = [...ru].filter((k) => !en.has(k))
const onlyEn = [...en].filter((k) => !ru.has(k))
for (const k of onlyRu) console.error(`нет в en.json: ${k}`)
for (const k of onlyEn) console.error(`нет в ru.json: ${k}`)
if (onlyRu.length || onlyEn.length) process.exit(1)
