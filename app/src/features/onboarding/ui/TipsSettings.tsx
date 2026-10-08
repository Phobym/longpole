import { useId } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { resetTips, setTipsEnabled, useTipsEnabled } from '../model/store'

/** «Показывать подсказки» и «Показать заново» (спека онбординга, § 4). Работает через стор: он сам пишет в настройки. */
export function TipsSettings() {
  const { t } = useTranslation()
  const id = useId()
  return (
    <div className="flex flex-wrap items-center gap-3">
      <label htmlFor={id} className="flex items-center gap-2 text-sm">
        <input id={id} type="checkbox" className="focus-ring" checked={useTipsEnabled()} onChange={(e) => setTipsEnabled(e.target.checked)} />
        {t('form.settings.tipsShow')}
      </label>
      <Button type="button" variant="outline" size="sm" onClick={resetTips}>
        {t('form.settings.tipsReset')}
      </Button>
    </div>
  )
}
