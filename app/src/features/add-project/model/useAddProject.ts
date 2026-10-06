import { useMutation, useQueryClient } from '@tanstack/react-query'
import { addProject, apiError, unwrap } from '../../../shared/api'
import { hostsQuery } from '../../../entities/host'
import { useOpenProject } from '../../../entities/project'
import { savedProjectsQuery } from '../../../entities/saved-project'
import { closeAddProject } from './dialog'

/** Разбор, проверка доступа и запись — в ядре; успех закрывает диалог и открывает проект. `lastInput` — ввод последней попытки (только ссылка или путь), для повтора после токена. */
export function useAddProject() {
  const queryClient = useQueryClient()
  const openProject = useOpenProject()
  const mutation = useMutation({
    mutationFn: (input: string) => unwrap(addProject(input)),
    onSuccess: async (saved) => {
      // токен нового хоста мог появиться по пути
      await Promise.all([queryClient.invalidateQueries(savedProjectsQuery), queryClient.invalidateQueries(hostsQuery)])
      closeAddProject()
      void openProject({ host: saved.host, project: saved.path, branch: undefined, name: saved.name })
    },
  })
  return { add: mutation.mutate, busy: mutation.isPending, error: apiError(mutation.error), lastInput: mutation.variables }
}
