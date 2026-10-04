import { useQueryClient } from '@tanstack/react-query'
import { clearHistory, removeHistory } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { confirm, showError } from '../../../shared/lib/dialogs'
import { historyKey } from '../../../entities/history-entry'

export function useHistoryActions() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  return {
    async remove(at: string) {
      const result = await removeHistory(at)
      if (result.ok) queryClient.setQueryData(historyKey, result.value)
      else await showError(t, result.error)
    },
    async clear() {
      if (!(await confirm(t, t('form.history.clearConfirm'), t('form.history.clear')))) return
      const result = await clearHistory()
      if (result.ok) queryClient.setQueryData(historyKey, [])
      else await showError(t, result.error)
    },
  }
}
