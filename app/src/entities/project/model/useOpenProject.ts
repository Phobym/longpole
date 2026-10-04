import { useNavigate } from '@tanstack/react-router'

declare module '@tanstack/history' {
  interface HistoryState {
    /** Имя проекта; из записи истории его нет — тогда заголовок `fullPath`. */
    name?: string
  }
}

export type ProjectTarget = { host: string; fullPath: string; ref?: string; name?: string; replace?: boolean }

/** Открывает экран «Проект»; ветка — в `?ref=`, имя — в `state` навигации. */
export function useOpenProject() {
  const navigate = useNavigate()
  return ({ host, fullPath, ref, name, replace }: ProjectTarget) =>
    navigate({ to: '/project/$host/$', params: { host, _splat: fullPath }, search: { ref }, state: { name }, replace })
}
