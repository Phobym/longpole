import { useMutation } from '@tanstack/react-query'
import { apiError, reportFileName, reportTemplate, unwrap, type Report } from '../../../shared/api'
import { capabilities } from '../../../shared/config'
import { useTranslation } from '../../../shared/i18n'
import { showError } from '../../../shared/lib/dialogs'
import { Button } from '../../../shared/ui/button'
import { renderReport } from '../model/render'

/**
 * «Скачать HTML» (спека поставки, § 10): тот же самодостаточный файл и то же имя, что у ⌘S в Desktop,
 * собирается во вкладке и уходит загрузкой. Тип `octet-stream`: файл скачивается и не открывается на origin сервера.
 */
export function DownloadButton({ id, report }: { id: number; report: Report }) {
  const { t } = useTranslation()
  const mutation = useMutation({
    mutationFn: async () => {
      const fileName = await unwrap(reportFileName(id))
      const html = renderReport(report, await unwrap(reportTemplate()))
      const url = URL.createObjectURL(new Blob([html], { type: 'application/octet-stream' }))
      const link = Object.assign(document.createElement('a'), { href: url, download: fileName })
      link.click()
      // загрузка уже началась: ссылка на Blob больше не нужна
      setTimeout(() => URL.revokeObjectURL(url))
    },
    onError: (e) => {
      const failure = apiError(e)
      if (failure) void showError(failure)
    },
  })
  if (capabilities.nativeSave) return null
  return (
    <Button type="button" variant="ghost" size="sm" disabled={mutation.isPending} onClick={() => mutation.mutate()}>
      {t('form.reportView.download')}
    </Button>
  )
}
