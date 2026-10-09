import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { oauthCancel, oauthLogin, type ApiError, type UserCode } from '../../../shared/api'
import { hostsQuery, normalizeHost, pickHost } from '../../../entities/host'

/** Вход через OAuth: ожидание, код GitHub, ошибка; после входа хост выбран, как после сохранения PAT. */
export function useOauthLogin(host: string) {
  const queryClient = useQueryClient()
  const [waiting, setWaiting] = useState(false)
  const [code, setCode] = useState<UserCode | null>(null)
  const [error, setError] = useState<ApiError | null>(null)
  const login = async (clientId: string | null) => {
    setWaiting(true)
    setError(null)
    const result = await oauthLogin(host, clientId, setCode).finally(() => {
      setWaiting(false)
      setCode(null)
    })
    if (!result.ok) {
      setError(result.error)
      return false
    }
    if (!result.value) return false
    await queryClient.invalidateQueries(hostsQuery)
    pickHost(normalizeHost(host))
    return true
  }
  const cancel = () => void oauthCancel(host)
  return { login, cancel, waiting, code, error }
}
