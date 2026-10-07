// Наборы ключей всех словарей должны совпадать с ru.json (вложенные ключи — через точку).
// Склонения (`key_one`, `key_few`, …) сравниваются по базовому ключу, а их формы — с категориями
// `Intl.PluralRules` языка: в русском `one`/`few`/`many`/`other`, в английском `one`/`other`, в китайском только `other`.
import { readdirSync, readFileSync } from 'node:fs'

const dir = new URL('../src/shared/i18n/locales/', import.meta.url)
const PLURAL = /_(zero|one|two|few|many|other)$/
const flat = (obj, prefix = '') =>
  Object.entries(obj).flatMap(([k, v]) => (v && typeof v === 'object' ? flat(v, `${prefix}${k}.`) : [`${prefix}${k}`]))

let failed = false
const fail = (msg) => {
  console.error(msg)
  failed = true
}

const load = (lang) => {
  const forms = new Map() // базовый ключ → формы склонения
  for (const key of flat(JSON.parse(readFileSync(new URL(`${lang}.json`, dir), 'utf8')))) {
    const m = PLURAL.exec(key)
    const base = m ? key.slice(0, m.index) : key
    forms.set(base, m ? [...(forms.get(base) ?? []), m[1]] : (forms.get(base) ?? []))
  }
  const expected = new Intl.PluralRules(lang).resolvedOptions().pluralCategories.sort().join()
  for (const [base, found] of forms) {
    if (found.length && found.sort().join() !== expected) fail(`${lang}.json: у ${base} формы ${found.sort()}, нужны ${expected}`)
  }
  return new Set(forms.keys())
}

const ru = load('ru')
for (const lang of readdirSync(dir).map((f) => f.replace(/\.json$/, '')).filter((l) => l !== 'ru')) {
  const keys = load(lang)
  for (const k of ru) if (!keys.has(k)) fail(`нет в ${lang}.json: ${k}`)
  for (const k of keys) if (!ru.has(k)) fail(`нет в ru.json: ${k} (есть в ${lang}.json)`)
}
if (failed) process.exit(1)
