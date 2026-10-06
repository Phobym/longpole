import { useNavigate, useParams, useRouter } from '@tanstack/react-router'
import { useEffect } from 'react'
import { apiError, setCurrentReport } from '../../shared/api'
import { useTranslation } from '../../shared/i18n'
import { showError } from '../../shared/lib/dialogs'
import { Button } from '../../shared/ui/button'
import { useReportById } from '../../entities/report'
import { ReportPage } from '../../pages/report'

/** Отчёт из памяти ядра; ядро узнаёт, что он на экране (⌘S). Вытесненный или битый id — ошибка и домой. */
export function ReportRoute() {
  const { t } = useTranslation()
  const { id } = useParams({ from: '/report/$id' })
  const reportId = Number(id)
  const navigate = useNavigate()
  const router = useRouter()
  const { data, error, isPending } = useReportById(reportId)

  useEffect(() => {
    void setCurrentReport(reportId)
    return () => void setCurrentReport(null)
  }, [reportId])

  const failure = apiError(error)
  useEffect(() => {
    if (!failure) return
    void showError(failure).then(() => navigate({ to: '/', replace: true }))
  }, [failure, navigate])

  const back = () => {
    if (router.history.canGoBack()) router.history.back()
    else void navigate({ to: '/' })
  }

  if (isPending || !data) return <p className="p-6 text-sm text-muted-foreground">{t('form.reportView.loading')}</p>
  const title = `${data.meta.project} · ${data.meta.label ?? t('report.allPipelines')}`
  return (
    <>
      <div className="sticky top-0 z-10 flex items-center gap-3 border-b bg-card px-4 py-2">
        <Button type="button" variant="ghost" size="sm" onClick={back}>
          {t('form.reportView.back')}
        </Button>
        <h1 translate="no" className="truncate text-sm font-semibold">
          {title}
        </h1>
      </div>
      <ReportPage report={data} embedded />
    </>
  )
}
