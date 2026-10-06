import type { Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { Segmented } from '../../../shared/ui/segmented'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: язык меняется на месте, состояние просмотра не теряется. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  return (
    <Segmented
      size="xs"
      label={t('report.language')}
      value={i18n.language as Locale}
      options={LOCALES.map((locale) => ({ value: locale, label: locale.toUpperCase(), lang: locale }))}
      onChange={(locale) => void changeLocale(locale)}
    />
  )
}
