import { useNavigate } from '@tanstack/react-router'

declare module '@tanstack/history' {
  interface HistoryState {
    /** Имя проекта; из записи истории его нет — тогда заголовок это `project`. */
    name?: string
  }
}

/** Проект на хосте и ветка, по которой смотрят его пайплайны (`undefined` — все ветки): общий словарь экрана «Проект». */
export type ProjectRef = { host: string; project: string; branch: string | undefined }

export type ProjectTarget = ProjectRef & { name?: string; replace?: boolean }

/** Открывает экран «Проект»; ветка — в `?ref=`, имя — в `state` навигации. */
export function useOpenProject() {
  const navigate = useNavigate()
  return ({ host, project, branch, name, replace }: ProjectTarget) =>
    navigate({ to: '/project/$host/$', params: { host, _splat: project }, search: { ref: branch }, state: { name }, replace })
}
