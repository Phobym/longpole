import { useQueryClient } from '@tanstack/react-query'
import { clearHistory, removeHistory } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { confirm, showError } from '../../../shared/lib/dialogs'
import { historyQuery } from '../../../entities/history-entry'

export function useHistoryActions() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  return {
    async remove(at: string) {
      const result = await removeHistory(at)
      if (result.ok) queryClient.setQueryData(historyQuery.queryKey, result.value)
      else await showError(result.error)
    },
    async clear() {
      if (!(await confirm(t('form.history.clearConfirm'), t('form.history.clear')))) return
      const result = await clearHistory()
      if (result.ok) queryClient.setQueryData(historyQuery.queryKey, [])
      else await showError(result.error)
    },
  }
}
