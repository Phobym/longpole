import i18next, { type TFunction } from 'i18next'
import { initReactI18next, useTranslation } from 'react-i18next'
import type { Locale } from '../api/schema/Locale'
import type { ApiError, IpcError } from '../api/api'
import type { ErrorBody } from '../api/schema/ErrorBody'
import de from './locales/de.json'
import en from './locales/en.json'
import es from './locales/es.json'
import fr from './locales/fr.json'
import it from './locales/it.json'
import ja from './locales/ja.json'
import ru from './locales/ru.json'
import zh from './locales/zh.json'

// Типизирует `t('errors.…')` по ru.json: ключ, которого нет в словаре, не проходит `tsc`.
declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation'
    resources: { translation: typeof ru }
  }
}

export { useTranslation }

/** Текущий язык интерфейса для `Intl`-форматирования из `shared/lib/format`. */
export const useLocale = (): Locale => useTranslation().i18n.language as Locale

/** Языки выпадающего списка: название — на самом языке, чтобы его нашёл тот, кто не читает текущий. */
export const LOCALES: { value: Locale; label: string }[] = [
  { value: 'ru', label: 'Русский' },
  { value: 'en', label: 'English' },
  { value: 'fr', label: 'Français' },
  { value: 'es', label: 'Español' },
  { value: 'de', label: 'Deutsch' },
  { value: 'it', label: 'Italiano' },
  { value: 'zh', label: '中文' },
  { value: 'ja', label: '日本語' },
]

/** Язык по тегу `navigator.language` (`zh-CN`, `fr`) — как `Locale::from_tag` в ядре; незнакомый — английский. */
export const browserLocale = (): Locale =>
  LOCALES.find(({ value }) => navigator.language.toLowerCase().startsWith(value))?.value ?? 'en'

/** `await` до первого рендера: форма — с `get_settings`, сохранённый отчёт — с языком браузера. */
export const initI18n = async (locale: Locale) => {
  document.documentElement.lang = locale
  i18next.on('languageChanged', (lng) => {
    document.documentElement.lang = lng
  })
  await i18next.use(initReactI18next).init({
    resources: Object.fromEntries(Object.entries({ ru, en, fr, es, de, it, zh, ja }).map(([l, translation]) => [l, { translation }])),
    lng: locale,
    fallbackLng: 'ru',
    // React экранирует сам; host и detail приходят чужими строками
    interpolation: { escapeValue: false },
  })
}

export const changeLocale = (locale: Locale) => i18next.changeLanguage(locale)

type ErrorCodes = ErrorBody['code'] | IpcError['code']

// tsc падает, если в ru.json нет перевода для кода ошибки ядра (совпадение en с ru сверяет check-locales)
void (ru.errors satisfies Record<ErrorCodes, string>)

type Failure = ApiError | { code: ErrorCodes; params: Record<string, string> }

/** Параметры для подстановки: `reset` лимита приходит как ISO 8601, человеку нужно местное время. */
const textParams = ({ code, params }: { code: ErrorCodes; params: Record<string, string> }) => {
  if (code !== 'rateLimited') return params
  const reset = new Date(params.reset ?? '')
  if (Number.isNaN(reset.getTime())) return params
  return { ...params, reset: reset.toLocaleTimeString(i18next.language, { hour: '2-digit', minute: '2-digit' }) }
}

/**
 * Текст ошибки по `code` и `params`; у `fields` — тексты всех полей подряд (вне `build` их не бывает).
 * `t` приведён к простой сигнатуре: для объединения ключей i18next требует все переменные всех строк сразу,
 * а ядро шлёт ровно нужные коду; сами коды проверяет `satisfies` выше.
 */
export const errorText = (t: TFunction, e: Failure): string =>
  'errors' in e
    ? Object.values(e.errors)
        .flatMap((body) => (body ? [errorText(t, body)] : []))
        .join(' ')
    : (t as unknown as (key: string, options: object) => string)(`errors.${e.code}`, textParams(e))

export const useErrorText = () => {
  const { t } = useTranslation()
  return (e: Failure) => errorText(t, e)
}
