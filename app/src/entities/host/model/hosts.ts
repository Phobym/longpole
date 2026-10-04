import { useQuery } from '@tanstack/react-query'
import { create } from 'zustand'
import { apiError, hosts, unwrap } from '../../../shared/api'

export const hostsKey = ['hosts'] as const

/** `undefined` — выбора не было, берём первый сохранённый хост; `null` — пункт «+ Добавить хост…». */
type Picked = string | null | undefined

const useHostStore = create<{ picked: Picked; pick: (host: Picked) => void }>((set) => ({
  picked: undefined,
  pick: (picked) => set({ picked }),
}))

/** Как `core::hosts::normalize_host`: без схемы и завершающего слэша; ядро хранит хост именно так. */
export const normalizeHost = (raw: string) =>
  raw
    .trim()
    .replace(/^https?:\/\//, '')
    .replace(/\/$/, '')

export const pickHost = (host: Picked) => useHostStore.getState().pick(host)

/** Текущий хост формы и то, что про него известно. `host === null` — «+ Добавить хост…». */
export function useHostState() {
  const { data: saved = [], isPending, error } = useQuery({ queryKey: hostsKey, queryFn: () => unwrap(hosts()) })
  const picked = useHostStore((s) => s.picked)
  const host = picked === undefined ? (saved[0] ?? null) : picked
  return { host, saved, hasToken: host !== null && saved.includes(host), ready: !isPending, error: apiError(error) }
}
