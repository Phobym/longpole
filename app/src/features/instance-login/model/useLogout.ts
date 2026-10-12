import { useQueryClient } from '@tanstack/react-query'
import { logout } from '../../../shared/api'
import { showError } from '../../../shared/lib/dialogs'
import { hostsQuery, resetHostPick } from '../../../entities/host'

/** «Выйти» у хоста Selfhosted: стирает вход только в него (спека, § 10). */
export function useLogout() {
  const queryClient = useQueryClient()
  return async (host: string) => {
    const result = await logout(host)
    if (!result.ok) return showError(result.error)
    await queryClient.invalidateQueries(hostsQuery)
    resetHostPick() // выбранным мог быть этот хост
  }
}
