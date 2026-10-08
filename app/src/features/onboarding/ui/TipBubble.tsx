import { Trans } from 'react-i18next'
import { useTranslation } from '../../../shared/i18n'
import { Giraffe } from '../../../shared/ui/giraffe'
import { closeTip, disableTips } from '../model/store'
import type { TipId } from '../model/tips'

const ACCENT = { b: <b className="font-semibold text-crit-fg" /> }
const action = 'cursor-pointer rounded-sm focus-ring hover:underline'

/** Содержимое подсказки: жираф слева, заголовок, текст с выделениями, «Понятно» и «Больше не показывать». */
export function TipBubble({ id, titleId }: { id: TipId; titleId: string }) {
  const { t } = useTranslation()
  return (
    <div className="flex gap-3">
      <Giraffe size={56} />
      <div className="flex min-w-0 flex-col gap-1">
        <p id={titleId} className="text-[13px] font-semibold">{t(`onboarding.${id}.title`)}</p>
        <p className="text-[12.5px] leading-[1.4]">
          <Trans i18nKey={`onboarding.${id}.text`} components={ACCENT} />
        </p>
        <div className="mt-1.5 flex gap-3 text-xs">
          <button type="button" className={`${action} font-semibold text-crit-fg`} onClick={closeTip}>
            {t('onboarding.ok')}
          </button>
          <button type="button" className={`${action} text-muted-foreground`} onClick={disableTips}>
            {t('onboarding.never')}
          </button>
        </div>
      </div>
    </div>
  )
}
