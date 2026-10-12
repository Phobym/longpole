import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { apiError, rebuild, report as reportById, unwrap, type Progress, type Report } from '../../../shared/api'
import { capabilities } from '../../../shared/config'
import { useTranslation } from '../../../shared/i18n'
import { showError } from '../../../shared/lib/dialogs'
import { Button } from '../../../shared/ui/button'
import { isFinished } from '../../../entities/report'
import { historyQuery } from '../../../entities/history-entry'

/**
 * «Перестроить» (спека поставки, § 10): отчёт по завершённому пайплайну строится заново мимо кэша сервера
 * и ложится под тот же id — адрес страницы не меняется. Новый отчёт получает `onRebuilt`.
 */
export function RebuildButton({ id, report, onRebuilt }: { id: number; report: Report; onRebuilt: (report: Report) => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [progress, setProgress] = useState<Progress | null>(null)
  const mutation = useMutation({
    mutationFn: async () => {
      setProgress(null)
      await unwrap(rebuild(id, setProgress))
      return unwrap(reportById(id))
    },
    onSuccess: (next) => {
      void queryClient.invalidateQueries(historyQuery)
      onRebuilt(next)
    },
    onError: (e) => {
      const failure = apiError(e)
      if (failure) void showError(failure)
    },
  })
  if (!capabilities.reportCache || !isFinished(report)) return null
  const busy = progress ? t('form.progress.step', progress) : t('form.progress.start')
  return (
    <Button type="button" variant="ghost" size="sm" className="ml-auto" disabled={mutation.isPending} onClick={() => mutation.mutate()}>
      {mutation.isPending ? busy : t('form.reportView.rebuild')}
    </Button>
  )
}
