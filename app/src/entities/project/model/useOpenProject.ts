import { useNavigate } from '@tanstack/react-router'

declare module '@tanstack/history' {
  interface HistoryState {
    /** Имя проекта; из записи истории его нет — тогда заголовок это `project`. */
    name?: string
  }
}

/** Проект на хосте, ветка (`undefined` — все ветки) и workflow GitHub (`undefined` — первый из списка): словарь экрана «Проект». */
export type ProjectRef = { host: string; project: string; branch: string | undefined; workflow?: string }

export type ProjectTarget = ProjectRef & { name?: string; replace?: boolean }

/** Открывает экран «Проект»; ветка — в `?ref=`, workflow — в `?workflow=`, имя — в `state` навигации. */
export function useOpenProject() {
  const navigate = useNavigate()
  return ({ host, project, branch, workflow, name, replace }: ProjectTarget) =>
    navigate({ to: '/project/$host/$', params: { host, _splat: project }, search: { ref: branch, workflow }, state: { name }, replace })
}
