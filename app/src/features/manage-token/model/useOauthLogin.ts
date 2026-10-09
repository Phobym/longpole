import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'
import { oauthCancel, oauthLogin, type ApiError, type UserCode } from '../../../shared/api'
import { hostsQuery, normalizeHost, pickHost } from '../../../entities/host'

/** Вход через OAuth: ожидание, код GitHub, ошибка; после входа хост выбран, как после сохранения PAT. */
export function useOauthLogin(host: string) {
  const queryClient = useQueryClient()
  const [waiting, setWaiting] = useState(false)
  const [code, setCode] = useState<UserCode | null>(null)
  const [error, setError] = useState<ApiError | null>(null)
  // Хост, с которого начался вход: поле хоста в форме можно менять во время ожидания, а отменять надо тот же хост
  const pending = useRef<string | null>(null)
  // Размонтирование (например, «Отмена» в настройках) не должно оставлять вход жить в фоне и молча сохранить токен
  useEffect(
    () => () => {
      if (pending.current) void oauthCancel(pending.current)
    },
    [],
  )
  const login = async (clientId: string | null) => {
    pending.current = host
    setWaiting(true)
    setCode(null)
    setError(null)
    const result = await oauthLogin(host, clientId, setCode).finally(() => {
      pending.current = null
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
  const cancel = () => {
    if (pending.current) void oauthCancel(pending.current)
  }
  return { login, cancel, waiting, code, error }
}
