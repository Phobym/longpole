import { queryOptions, useQuery, useQueryClient } from '@tanstack/react-query'
import { apiError, removeProject, savedProjects, unwrap, type ProjectRef } from '../../../shared/api'
import { showError } from '../../../shared/lib/dialogs'

export const savedProjectsQuery = queryOptions({ queryKey: ['savedProjects'], queryFn: () => unwrap(savedProjects()) })

export function useSavedProjects() {
  const { data, isPending, error } = useQuery(savedProjectsQuery)
  return { projects: data ?? [], ready: !isPending, error: apiError(error) }
}

/** Убирает проект из списка; ядро заодно забывает его как «последний». */
export function useRemoveProject() {
  const queryClient = useQueryClient()
  return async (project: ProjectRef) => {
    const result = await removeProject(project)
    if (result.ok) queryClient.setQueryData(savedProjectsQuery.queryKey, result.value)
    else await showError(result.error)
  }
}
