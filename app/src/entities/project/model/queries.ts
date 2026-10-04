import { keepPreviousData, useInfiniteQuery, useQuery } from '@tanstack/react-query'
import { branches, projects, unwrap } from '../../../shared/api'

// Гонки ответов закрыты ключами: поиск или хост сменились — это другой запрос, чужой ответ в список не попадёт.
export const useProjects = (host: string, search: string) =>
  useInfiniteQuery({
    queryKey: ['projects', host, search],
    queryFn: ({ pageParam }) => unwrap(projects(host, search, pageParam)),
    initialPageParam: null as string | null,
    getNextPageParam: (last) => last.next,
  })

export const useBranches = (host: string, project: string, search: string) =>
  useQuery({
    queryKey: ['branches', host, project, search],
    queryFn: () => unwrap(branches(host, project, search)),
    placeholderData: keepPreviousData, // подсказки не мигают, пока печатаешь
  })
