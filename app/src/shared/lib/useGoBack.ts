import { useNavigate, useRouter } from '@tanstack/react-router'
import { useCallback } from 'react'

/** Назад по истории, а без неё — домой. Ссылка стабильна — годится в зависимости эффекта. */
export function useGoBack() {
  const router = useRouter()
  const navigate = useNavigate()
  return useCallback(() => {
    if (router.history.canGoBack()) router.history.back()
    else void navigate({ to: '/' })
  }, [router, navigate])
}
