import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { build, type Form, type Progress, apiError, unwrap } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { historyQuery } from './history'

/**
 * Одна сборка отчёта со своим `Channel`: прогресс получает только тот, кто её запустил.
 * Лежит в `entities/history-entry`, а не в фиче: успешная сборка добавляет запись истории, а потребителей три
 * (ссылка, агрегат, строка пайплайна), и фичи не могут импортировать друг друга.
 */
export function useBuild() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [progress, setProgress] = useState<Progress | null>(null)
  const mutation = useMutation({
    mutationFn: (form: Form) => {
      setProgress(null)
      return unwrap(build(form, setProgress))
    },
    // успех ведёт на экран отчёта: прогресс строки или кнопки гаснет вместе с размонтированием
    onSuccess: (id) => {
      void queryClient.invalidateQueries(historyQuery)
      void navigate({ to: '/report/$id', params: { id: String(id) } })
    },
  })
  return {
    start: mutation.mutate,
    busy: mutation.isPending,
    /** «Загружаю N из M…» (или «Загружаю…», пока прогресса нет). */
    progressLabel: progress ? t('form.progress.step', progress) : t('form.progress.start'),
    error: apiError(mutation.error),
  }
}
