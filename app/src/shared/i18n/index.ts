import i18next, { type TFunction } from 'i18next'
import { initReactI18next, useTranslation } from 'react-i18next'
import type { Locale } from '../api/schema/Locale'
import type { ApiError, IpcError } from '../api/api'
import type { ErrorBody } from '../api/schema/ErrorBody'
import en from './locales/en.json'
import ru from './locales/ru.json'

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

/** `await` до первого рендера: форма — с `get_locale`, отчёт — с `meta.locale` или языком браузера. */
export const initI18n = async (locale: Locale) => {
  document.documentElement.lang = locale
  i18next.on('languageChanged', (lng) => {
    document.documentElement.lang = lng
  })
  await i18next.use(initReactI18next).init({
    resources: { ru: { translation: ru }, en: { translation: en } },
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
    : (t as unknown as (key: string, options: object) => string)(`errors.${e.code}`, e.params)

export const useErrorText = () => {
  const { t } = useTranslation()
  return (e: Failure) => errorText(t, e)
}
