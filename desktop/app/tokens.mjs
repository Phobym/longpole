import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

export function createTokenStore({ file, crypto }) {
  const load = async () => {
    try {
      return JSON.parse(await readFile(file, 'utf8'))
    } catch (err) {
      if (err.code === 'ENOENT') return {}
      throw err
    }
  }
  const save = async (data) => {
    await mkdir(dirname(file), { recursive: true })
    await writeFile(file, JSON.stringify(data, null, 2), { mode: 0o600 })
  }
  return {
    async hosts() {
      return Object.keys(await load()).sort()
    },
    async get(host) {
      const value = (await load())[host]
      return value ? crypto.decryptString(Buffer.from(value, 'base64')) : null
    },
    async set(host, token) {
      if (!crypto.isEncryptionAvailable()) {
        throw new Error('Системная связка ключей недоступна: задай GITLAB_TOKEN и GITLAB_HOST или установи libsecret')
      }
      if (!String(token ?? '').trim()) throw new Error('Пустой токен')
      const data = await load()
      data[host] = crypto.encryptString(String(token).trim()).toString('base64')
      await save(data)
    },
    async remove(host) {
      const data = await load()
      delete data[host]
      await save(data)
    },
  }
}
