import { cp, rm } from 'node:fs/promises'

const from = new URL('../../src/', import.meta.url)
const to = new URL('../app/core/', import.meta.url)
await rm(to, { recursive: true, force: true })
await cp(from, to, { recursive: true })
