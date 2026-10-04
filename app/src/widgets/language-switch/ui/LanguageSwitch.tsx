import { setLocale, type Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { showError } from '../../../shared/lib/dialogs'
import { Button } from '../../../shared/ui/button'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: форма перерисовывается сразу, ядро запоминает язык и пересобирает меню. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  const choose = async (locale: Locale) => {
    await changeLocale(locale)
    const result = await setLocale(locale)
    if (!result.ok) await showError(result.error)
  }
  return (
    <div role="group" aria-label={t('form.language')} className="flex gap-0.5">
      {LOCALES.map((locale) => (
        <Button
          key={locale}
          variant={i18n.language === locale ? 'secondary' : 'ghost'}
          size="sm"
          className="h-7 px-2.5 text-xs"
          lang={locale}
          aria-pressed={i18n.language === locale}
          onClick={() => void choose(locale)}
        >
          {locale.toUpperCase()}
        </Button>
      ))}
    </div>
  )
}
