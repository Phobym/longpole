import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { createTokenStore } from '../app/tokens.mjs'

const fakeCrypto = (available = true) => ({
  isEncryptionAvailable: () => available,
  encryptString: (s) => Buffer.from(`enc:${s}`),
  decryptString: (b) => b.toString().replace(/^enc:/, ''),
})
const tmpFile = async () => join(await mkdtemp(join(tmpdir(), 'pt-tokens-')), 'tokens.json')

test('set/get/hosts/remove, в файле нет открытого токена', async () => {
  const file = await tmpFile()
  const store = createTokenStore({ file, crypto: fakeCrypto() })
  assert.deepEqual(await store.hosts(), [])
  await store.set('h.example', 'secret-1')
  assert.equal(await store.get('h.example'), 'secret-1')
  assert.deepEqual(await store.hosts(), ['h.example'])
  assert.ok(!(await readFile(file, 'utf8')).includes('secret-1'))
  await store.remove('h.example')
  assert.equal(await store.get('h.example'), null)
})

test('без шифрования сохранение запрещено понятной ошибкой', async () => {
  const store = createTokenStore({ file: await tmpFile(), crypto: fakeCrypto(false) })
  await assert.rejects(store.set('h.example', 'x'), /связка ключей недоступна/)
})

test('пустой токен не сохраняется', async () => {
  const store = createTokenStore({ file: await tmpFile(), crypto: fakeCrypto() })
  await assert.rejects(store.set('h.example', '   '), /Пустой токен/)
})
