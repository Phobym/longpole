import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname } from 'node:path'

export function createHistory({ file, limit = 20, now = () => new Date() }) {
  const list = async () => {
    try {
      return JSON.parse(await readFile(file, 'utf8'))
    } catch (err) {
      if (err.code === 'ENOENT') return []
      throw err
    }
  }
  const key = (e) => JSON.stringify([e.host, e.request])
  return {
    list,
    async add({ host, form, request, label }) {
      const entry = { at: now().toISOString(), host, form, request, label }
      const next = [entry, ...(await list()).filter((e) => key(e) !== key(entry))].slice(0, limit)
      await mkdir(dirname(file), { recursive: true })
      await writeFile(file, JSON.stringify(next, null, 2))
      return next
    },
  }
}
