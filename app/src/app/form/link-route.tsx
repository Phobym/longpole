import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useLocation } from '@tanstack/react-router'
import { useState } from 'react'
import { apiError, build, login, report, unwrap, type ApiError, type BuildProgress, type Report } from '../../shared/api'
import { progressText, useErrorText, useTranslation } from '../../shared/i18n'
import { Button } from '../../shared/ui/button'
import { historyQuery, linkForm } from '../../entities/history-entry'
import { reportLink } from './link'
import { ReportLoading, ReportScreen } from './report-route'

/**
 * Отчёт по ссылке (спека поставки, § 8): адрес страницы и есть отчёт, на `/report/<id>` он не меняется.
 * Без входа в хост транспорт сам уводит на вход с возвратом сюда; если вход не удался — экран «Войди в <домен>».
 */
export function ReportLinkRoute() {
  const { t } = useTranslation()
  const pathname = useLocation({ select: (l) => l.pathname })
  const link = reportLink(pathname)
  const queryClient = useQueryClient()
  const queryKey = ['reportLink', link?.url]
  const [progress, setProgress] = useState<BuildProgress | null>(null)
  // запрос, а не мутация: двойной эффект StrictMode и возврат «Назад» не строят отчёт заново
  const { data, error } = useQuery({
    queryKey,
    queryFn: async () => {
      setProgress(null)
      const id = await unwrap(build(linkForm(link!.url), setProgress))
      void queryClient.invalidateQueries(historyQuery)
      return { id, report: await unwrap(report(id)) }
    },
    enabled: link !== null,
    staleTime: Infinity,
  })

  if (!link) return <LinkFailure error={{ kind: 'message', code: 'invalidLink', params: {} }} />
  const failure = apiError(error)
  // сброс, а не refetch: у refetch ошибка висит до ответа, а нужны «В очереди…» и прогресс
  if (failure) return <LinkFailure error={failure} onRetry={() => void queryClient.resetQueries({ queryKey })} />
  if (!data) return <ReportLoading text={progressText(t, progress)} />
  const onRebuilt = (next: Report) => queryClient.setQueryData(queryKey, { id: data.id, report: next })
  return <ReportScreen id={data.id} report={data.report} onRebuilt={onRebuilt} />
}

function LinkFailure({ error, onRetry }: { error: ApiError; onRetry?: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const host = error.kind === 'message' && error.code === 'authRequired' ? error.params.host : undefined
  return (
    <div className="flex flex-col items-start gap-3 p-6 text-sm">
      <p role="alert">{host ? t('form.reportLink.signIn', { host }) : errorText(error)}</p>
      {host ? (
        <Button type="button" onClick={() => login(host)}>
          {t('form.settings.signIn')}
        </Button>
      ) : error.kind === 'message' && error.code === 'buildTimeout' && onRetry ? (
        <Button type="button" onClick={onRetry}>
          {t('form.reportLink.retry')}
        </Button>
      ) : (
        <Link to="/" className="text-link hover:underline">
          {t('form.reportLink.home')}
        </Link>
      )}
    </div>
  )
}
