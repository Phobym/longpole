import { useNavigate, useParams } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
import { apiError, setCurrentReport } from '../../shared/api'
import { useTranslation } from '../../shared/i18n'
import { useGoBack } from '../../shared/lib/useGoBack'
import { showError } from '../../shared/lib/dialogs'
import { Button } from '../../shared/ui/button'
import { isReportId, useReportById } from '../../entities/report'
import { ReportPage } from '../../pages/report'
import { Giraffe } from '../../shared/ui/giraffe'

/** Отчёт из памяти ядра; ядро узнаёт, что он на экране (⌘S). Вытесненный id — ошибка и домой, битый (`/report/abc`) — сразу домой без диалога. */
export function ReportRoute() {
  const { t } = useTranslation()
  const { id } = useParams({ from: '/report/$id' })
  const reportId = Number(id)
  const navigate = useNavigate()
  const back = useGoBack()
  const valid = isReportId(reportId)
  const { data, error, isPending } = useReportById(reportId)

  useEffect(() => {
    if (!valid) return
    void setCurrentReport(reportId)
    return () => void setCurrentReport(null)
  }, [reportId, valid])

  // Битый id (`/report/abc`): показывать нечего, сразу домой.
  useEffect(() => {
    if (!valid) void navigate({ to: '/', replace: true })
  }, [valid, navigate])

  // `apiError` для чужих ошибок каждый раз отдаёт новый объект, поэтому диалог — раз за монтирование (и один при двойном эффекте StrictMode).
  const reported = useRef(false)
  useEffect(() => {
    const failure = apiError(error)
    if (!failure || reported.current) return
    reported.current = true
    void showError(failure).then(() => navigate({ to: '/', replace: true }))
  }, [error, navigate])

  if (!valid) return null
  if (isPending || !data) {
    return (
      <p className="flex items-center gap-3 p-6 text-sm text-muted-foreground">
        <Giraffe pose="loading" size={72} />
        {t('form.reportView.loading')}
      </p>
    )
  }
  const title = `${data.meta.project} · ${data.meta.label ?? t('report.allPipelines')}`
  return (
    <>
      {/* не sticky: липкая ось водопада и панель деталей пинятся к верху того же скролла */}
      <div className="flex items-center gap-3 border-b bg-chrome px-4 py-1.5">
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
