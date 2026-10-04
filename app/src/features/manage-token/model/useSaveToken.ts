import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { setToken, type ApiError } from '../../../shared/api'
import { hostsKey, normalizeHost, pickHost } from '../../../entities/host'

/** Не `useMutation`: он держит аргументы (а с ними токен) в `variables` и в кеше мутаций ещё минуты после сохранения. */
export function useSaveToken() {
  const queryClient = useQueryClient()
  const [error, setError] = useState<ApiError | null>(null)
  const save = async (host: string, token: string) => {
    const result = await setToken(host, token)
    setError(result.ok ? null : result.error)
    if (!result.ok) return
    await queryClient.invalidateQueries({ queryKey: hostsKey })
    pickHost(normalizeHost(host))
  }
  return { save, error }
}
