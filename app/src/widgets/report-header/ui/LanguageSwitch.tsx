import type { Locale } from '../../../shared/api'
import { changeLocale, LOCALES, useTranslation } from '../../../shared/i18n'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../../shared/ui/select'

/** Список языков: язык меняется на месте, состояние просмотра не теряется. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  return (
    <Select value={i18n.language} onValueChange={(locale) => void changeLocale(locale as Locale)}>
      <SelectTrigger aria-label={t('report.language')} className="h-5 w-auto px-2 text-[11px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent align="end">
        {LOCALES.map(({ value, label }) => (
          <SelectItem key={value} value={value} lang={value}>
            {label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
