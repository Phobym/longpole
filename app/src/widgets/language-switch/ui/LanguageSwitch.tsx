import { useUpdateSettings } from '../../../entities/settings'
import type { Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { Segmented } from '../../../shared/ui/segmented'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: форма перерисовывается сразу, ядро запоминает язык и пересобирает меню. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  const update = useUpdateSettings()
  const choose = async (locale: Locale) => {
    await changeLocale(locale)
    await update({ locale })
  }
  return (
    <Segmented
      label={t('form.language')}
      value={i18n.language as Locale}
      options={LOCALES.map((locale) => ({ value: locale, label: locale.toUpperCase(), lang: locale }))}
      onChange={(locale) => void choose(locale)}
    />
  )
}
