import type { Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: язык меняется на месте, состояние просмотра не теряется. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  return (
    <div role="group" aria-label={t('report.language')} className="flex gap-0.5">
      {LOCALES.map((locale) => (
        <Button
          key={locale}
          variant={i18n.language === locale ? 'secondary' : 'ghost'}
          className="h-6 rounded-full px-2 text-[11px]"
          lang={locale}
          aria-pressed={i18n.language === locale}
          onClick={() => void changeLocale(locale)}
        >
          {locale.toUpperCase()}
        </Button>
      ))}
    </div>
  )
}
