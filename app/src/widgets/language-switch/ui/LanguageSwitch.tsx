import { useUpdateSettings } from '../../../entities/settings'
import type { Locale } from '../../../shared/api'
import { changeLocale, LOCALES, useTranslation } from '../../../shared/i18n'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../../shared/ui/select'

/** Список языков: форма перерисовывается сразу, ядро запоминает язык и пересобирает меню. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  const update = useUpdateSettings()
  const choose = async (locale: Locale) => {
    await changeLocale(locale)
    await update({ locale })
  }
  return (
    <Select value={i18n.language} onValueChange={(locale) => void choose(locale as Locale)}>
      <SelectTrigger aria-label={t('form.language')} className="w-40">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {LOCALES.map(({ value, label }) => (
          <SelectItem key={value} value={value} lang={value}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
