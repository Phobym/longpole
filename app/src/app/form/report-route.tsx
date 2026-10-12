import { useQueryClient } from '@tanstack/react-query'
import { useNavigate, useParams } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
import { apiError, setCurrentReport, type Report } from '../../shared/api'
import { useTranslation } from '../../shared/i18n'
import { useGoBack } from '../../shared/lib/useGoBack'
import { showError } from '../../shared/lib/dialogs'
import { Button } from '../../shared/ui/button'
import { isReportId, useReportById } from '../../entities/report'
import { ReportPage } from '../../pages/report'
import { markLeftReport } from '../../features/onboarding'
import { RebuildButton } from '../../features/rebuild-report'
import { Giraffe } from '../../shared/ui/giraffe'

/** Отчёт из памяти ядра; ядро узнаёт, что он на экране (⌘S). Вытесненный id — ошибка и домой, битый (`/report/abc`) — сразу домой без диалога. */
export function ReportRoute() {
  const { t } = useTranslation()
  const { id } = useParams({ from: '/report/$id' })
  const reportId = Number(id)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const valid = isReportId(reportId)
  const { data, error, isPending } = useReportById(reportId)

  useEffect(() => {
    if (!valid) return
    void setCurrentReport(reportId)
    return () => {
      void setCurrentReport(null)
      // уход с отчёта — событие для подсказки поля ссылки в шапке
      markLeftReport()
    }
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
  if (isPending || !data) return <ReportLoading text={t('form.reportView.loading')} />
  return <ReportScreen id={reportId} report={data} onRebuilt={(next) => queryClient.setQueryData(['report', reportId], next)} />
}

export const ReportLoading = ({ text }: { text: string }) => (
  <p className="flex items-center gap-3 p-6 text-sm text-muted-foreground">
    <Giraffe pose="loading" size={72} />
    {text}
  </p>
)

/** Отчёт на весь экран с полосой «Назад», заголовком и «Перестроить»; `id` — отчёт в памяти транспорта. */
export function ReportScreen({ id, report, onRebuilt }: { id: number; report: Report; onRebuilt: (report: Report) => void }) {
  const { t } = useTranslation()
  const back = useGoBack()
  const title = `${report.meta.project} · ${report.meta.label ?? t('report.allPipelines')}`
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
        <RebuildButton id={id} report={report} onRebuilt={onRebuilt} />
      </div>
      <ReportPage report={report} embedded />
    </>
  )
}
