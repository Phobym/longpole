import { useQueryClient } from '@tanstack/react-query'
import { removeToken } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { confirm, showError } from '../../../shared/lib/dialogs'
import { hostsQuery, resetHostPick } from '../../../entities/host'

/** Подтверждение через plugin-dialog: нативный `confirm` в Tauri 2 на macOS не работает. */
export function useRemoveToken() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  return async (host: string) => {
    if (!(await confirm(t('form.host.removeConfirm', { host }), t('form.host.removeToken')))) return
    const result = await removeToken(host)
    if (!result.ok) return showError(result.error)
    await queryClient.invalidateQueries(hostsQuery)
    resetHostPick() // первый из оставшихся или «+ Добавить хост…»
  }
}
