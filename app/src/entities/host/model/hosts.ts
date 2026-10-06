import { queryOptions, useQuery } from '@tanstack/react-query'
import { create } from 'zustand'
import { apiError, hosts, unwrap } from '../../../shared/api'

export const hostsQuery = queryOptions({ queryKey: ['hosts'], queryFn: () => unwrap(hosts()) })

/** `auto` — выбора не было, берём первый сохранённый хост; `add` — пункт «+ Добавить хост…». */
type Picked = { kind: 'auto' } | { kind: 'add' } | { kind: 'host'; host: string }

const useHostStore = create<{ picked: Picked }>(() => ({ picked: { kind: 'auto' } }))

const pick = (picked: Picked) => useHostStore.setState({ picked })
export const pickHost = (host: string) => pick({ kind: 'host', host })
export const pickAddHost = () => pick({ kind: 'add' })
export const resetHostPick = () => pick({ kind: 'auto' })

/** Как `core::hosts::normalize_host`: без схемы и завершающего слэша; ядро хранит хост именно так. */
export const normalizeHost = (raw: string) =>
  raw
    .trim()
    .replace(/^https?:\/\//, '')
    .replace(/\/$/, '')

/** Текущий хост формы и то, что про него известно. `host === null` — «+ Добавить хост…». */
export function useHostState() {
  const { data: saved = [], isPending, error } = useQuery(hostsQuery)
  const picked = useHostStore((s) => s.picked)
  const host = picked.kind === 'auto' ? (saved[0]?.host ?? null) : picked.kind === 'host' ? picked.host : null
  return {
    host,
    saved,
    /** Текущий хост, если для него сохранён токен. */
    tokenHost: host !== null && saved.some((h) => h.host === host) ? host : null,
    ready: !isPending,
    error: apiError(error),
  }
}
