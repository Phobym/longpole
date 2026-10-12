import assert from 'node:assert/strict'
import { test } from 'node:test'
import { build, type Rollup } from 'vite'

// Код Tauri в бандле узнаётся по строкам моста и плагина диалогов: имена пакетов сборка не сохраняет
const TAURI = /__TAURI_INTERNALS__|plugin:dialog/

async function bundle(delivery: 'desktop' | 'selfhosted'): Promise<string> {
  process.env.VITE_DELIVERY = delivery
  try {
    const out = (await build({ configFile: 'vite.config.ts', logLevel: 'silent', build: { write: false } })) as Rollup.RollupOutput
    return out.output.map((f) => (f.type === 'chunk' ? f.code : '')).join('\n')
  } finally {
    delete process.env.VITE_DELIVERY
  }
}

test('в Selfhosted-бандле нет @tauri-apps/*, в Desktop — есть', { timeout: 120_000 }, async () => {
  assert.match(await bundle('desktop'), TAURI)
  assert.doesNotMatch(await bundle('selfhosted'), TAURI)
})
