import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { build, type Form, type Progress, apiError, unwrap } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { historyKey } from './history'

/**
 * Одна сборка отчёта со своим `Channel`: прогресс получает только тот, кто её запустил.
 * Лежит в `entities/history-entry`, а не в фиче: успешная сборка добавляет запись истории, а потребителей три
 * (ссылка, агрегат, строка пайплайна), и фичи не могут импортировать друг друга.
 */
export function useBuild() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [progress, setProgress] = useState<Progress | null>(null)
  const mutation = useMutation({
    mutationFn: (form: Form) => {
      setProgress(null)
      return unwrap(build(form, setProgress))
    },
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: historyKey }),
  })
  return {
    start: mutation.mutate,
    /** «Загружаю N из M…», пока идёт сборка; иначе `null`. */
    progress: mutation.isPending ? (progress ? t('form.progress.step', progress) : t('form.progress.start')) : null,
    error: apiError(mutation.error),
  }
}
