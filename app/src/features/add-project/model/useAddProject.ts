import { useMutation, useQueryClient } from '@tanstack/react-query'
import { addProject, apiError, unwrap } from '../../../shared/api'
import { hostsQuery } from '../../../entities/host'
import { useOpenProject } from '../../../entities/project'
import { savedProjectsQuery } from '../../../entities/saved-project'
import { closeAddProject } from './dialog'

/** Разбор, проверка доступа и запись — в ядре; успех закрывает диалог и открывает проект. */
export function useAddProject() {
  const queryClient = useQueryClient()
  const openProject = useOpenProject()
  const mutation = useMutation({
    mutationFn: (input: string) => unwrap(addProject(input)),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries(savedProjectsQuery)
      await queryClient.invalidateQueries(hostsQuery) // токен нового хоста мог появиться по пути
      closeAddProject()
      void openProject({ host: saved.host, project: saved.path, branch: undefined, name: saved.name })
    },
  })
  return { add: mutation.mutate, busy: mutation.isPending, error: apiError(mutation.error), reset: mutation.reset }
}
