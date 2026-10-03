import type { Locale } from '../api/schema/Locale'

const perLocale = <T>(make: (locale: Locale) => T) => {
  const cache = new Map<Locale, T>()
  return (locale: Locale): T => {
    let found = cache.get(locale)
    if (!found) cache.set(locale, (found = make(locale)))
    return found
  }
}

const dateFmt = perLocale((l) => new Intl.DateTimeFormat(l, { dateStyle: 'short', timeStyle: 'short' }))
const relativeFmt = perLocale((l) => new Intl.RelativeTimeFormat(l, { numeric: 'auto' }))
const numberFmt = perLocale((l) => new Intl.NumberFormat(l))

// идиом MDN для Intl.RelativeTimeFormat: подбирает наибольшую подходящую единицу
const RELATIVE_DIVISIONS: { amount: number; unit: Intl.RelativeTimeFormatUnit }[] = [
  { amount: 60, unit: 'second' },
  { amount: 60, unit: 'minute' },
  { amount: 24, unit: 'hour' },
  { amount: 7, unit: 'day' },
  { amount: 4.34524, unit: 'week' },
  { amount: 12, unit: 'month' },
]

const pad2 = (n: number) => String(n).padStart(2, '0')

// минуты и часы у обеих функций длительности одинаковы; различаются только секунды и меньше
function minutesHours(s: number): string | null {
  if (s < 60) return null
  const m = Math.floor(s / 60)
  return m < 60 ? `${m}m${pad2(Math.floor(s % 60))}s` : `${Math.floor(m / 60)}h${pad2(m % 60)}m`
}

/** Форма: `45s`, `12m05s`, `1h02m`, «—» без значения. */
export function durationShort(ms: number | null): string {
  if (ms == null) return '—'
  const s = ms / 1000
  return minutesHours(s) ?? `${Math.round(s)}s`
}

/** Отчёт: `850ms`, `12.3s`, `1m05s`, `0s`, «—» без значения. */
export function duration(ms: number | null): string {
  if (ms == null) return '—'
  if (ms === 0) return '0s'
  if (ms < 1000) return `${Math.round(ms)}ms`
  const s = ms / 1000
  return minutesHours(s) ?? `${s.toFixed(1)}s`
}

export function relativeTime(iso: string, locale: Locale): string {
  let diff = (new Date(iso).getTime() - Date.now()) / 1000
  for (const { amount, unit } of RELATIVE_DIVISIONS) {
    if (Math.abs(diff) < amount) return relativeFmt(locale).format(Math.round(diff), unit)
    diff /= amount
  }
  return relativeFmt(locale).format(Math.round(diff), 'year')
}

export const dateTime = (iso: string, locale: Locale): string => dateFmt(locale).format(new Date(iso))

export const number = (n: number, locale: Locale): string => numberFmt(locale).format(n)
