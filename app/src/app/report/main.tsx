import i18next from 'i18next'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { ReportPage } from '../../pages/report'
import { fixtures, type Locale, type Report } from '../../shared/api'
import { initI18n } from '../../shared/i18n'
import '../../shared/ui/theme.css'

// В окне приложения язык — из отчёта; сохранённый файл в браузере открывается на языке браузера.
const inApp = location.protocol === 'report:' || location.hostname === 'report.localhost'
const browserLocale = (): Locale => (navigator.language.toLowerCase().startsWith('ru') ? 'ru' : 'en')

async function readReport(): Promise<Report> {
  // dev-сервер: данных от Rust нет, берём эталонные фикстуры (?fixture=single|aggregate)
  if (import.meta.env.DEV) {
    const name = new URLSearchParams(location.search).get('fixture') === 'single' ? 'single' : 'aggregate'
    return (await fixtures[name]()).default as Report
  }
  // данные подставляет Rust (core::render); в шаблоне блок пустой
  return JSON.parse(document.getElementById('data')?.textContent || 'null') ?? Promise.reject(new Error('в отчёте нет данных'))
}

async function main() {
  const root = document.getElementById('root')
  if (!root) throw new Error('нет #root в report.html')
  let report: Report
  try {
    report = await readReport()
  } catch (e) {
    await initI18n(browserLocale())
    root.textContent = i18next.t('report.loadFailed', { detail: String(e) })
    return
  }
  const locale = inApp ? report.meta.locale : browserLocale()
  await initI18n(locale)
  document.documentElement.lang = locale
  createRoot(root).render(
    <StrictMode>
      <ReportPage report={report} />
    </StrictMode>,
  )
}

main().catch((e) => {
  document.body.textContent = String(e)
})
