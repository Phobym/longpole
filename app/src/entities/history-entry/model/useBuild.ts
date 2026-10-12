import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { build, type BuildProgress, type Form, apiError, messageError, unwrap } from '../../../shared/api'
import { progressText, useTranslation } from '../../../shared/i18n'
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
  const [progress, setProgress] = useState<BuildProgress | null>(null)
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
  const error = apiError(mutation.error)
  return {
    start: mutation.mutate,
    busy: mutation.isPending,
    /** сырой прогресс для жирафа загрузки; `null`, пока ядро его не прислало */
    progress: progress === 'queued' ? null : progress,
    /** «В очереди…», «Загружаю N из M…» (или «Загружаю…», пока прогресса нет). */
    progressLabel: progressText(t, progress),
    error,
    /** «Повторить» после `buildTimeout` — та же форма; при других ошибках `null`. */
    retry: messageError(error)?.code === 'buildTimeout' ? () => mutation.mutate(mutation.variables!) : null,
  }
}
