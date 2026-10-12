import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { test } from 'node:test'
import { build, type Rollup } from 'vite'

// Код Tauri в бандле узнаётся по строкам моста и плагина диалогов: имена пакетов сборка не сохраняет
const TAURI = /__TAURI_INTERNALS__|plugin:dialog/

async function bundle(delivery: 'desktop' | 'selfhosted'): Promise<Rollup.OutputBundle[string][]> {
  process.env.VITE_DELIVERY = delivery
  try {
    const out = (await build({ configFile: 'vite.config.ts', logLevel: 'silent', build: { write: false } })) as Rollup.RollupOutput
    return out.output
  } finally {
    delete process.env.VITE_DELIVERY
  }
}
const code = (files: Rollup.OutputBundle[string][]) => files.map((f) => (f.type === 'chunk' ? f.code : '')).join('\n')
const names = (files: Rollup.OutputBundle[string][]) => files.map((f) => f.fileName)

test('в Selfhosted-бандле нет @tauri-apps/*, в Desktop — есть; шаблон «Скачать HTML» — только в Selfhosted', { timeout: 120_000 }, async () => {
  const desktop = await bundle('desktop')
  const selfhosted = await bundle('selfhosted')
  assert.match(code(desktop), TAURI)
  assert.doesNotMatch(code(selfhosted), TAURI)
  assert.ok(!names(desktop).includes('report-template.txt'))
  // шаблон собирает `npm run build` первым; тест до сборки его не застаёт
  assert.equal(names(selfhosted).includes('report-template.txt'), existsSync('dist-report/report.html'))
})
