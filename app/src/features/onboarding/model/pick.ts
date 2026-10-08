/** Выбор подсказки — чистая функция без импортов: её гоняет `node --test` без сборки (спека онбординга, § 4). */

export type TipDef<Id extends string = string> = {
  id: Id
  /** `enter` — готова, пока открыт экран или элемент; `event` — в момент действия пользователя */
  kind: 'enter' | 'event'
  /** показывается только после этой */
  after?: Id
}

export type PickInput<Id extends string> = {
  /** реестр в порядке приоритета */
  order: readonly TipDef<Id>[]
  ready: readonly Id[]
  seen: readonly string[]
  enabled: boolean
  /** `enter` за этот визит экрана уже была */
  enterShown: boolean
  active: Id | null
}

/** `event` важнее `enter`: она отвечает на действие прямо сейчас; `enter` — не больше одной за визит. */
export function pickTip<Id extends string>({ order, ready, seen, enabled, enterShown, active }: PickInput<Id>): Id | null {
  if (!enabled || active !== null) return null
  const candidates = order.filter((tip) => ready.includes(tip.id) && !seen.includes(tip.id) && (tip.after === undefined || seen.includes(tip.after)))
  const event = candidates.find((tip) => tip.kind === 'event')
  if (event) return event.id
  if (enterShown) return null
  return candidates.find((tip) => tip.kind === 'enter')?.id ?? null
}

/** Подсказка якоря с несколькими кандидатами: открытая держится, пока открыта, иначе — первая непросмотренная по порядку. */
export function anchorTip<Id extends string>(candidates: readonly Id[], seen: readonly string[], active: Id | null): Id | null {
  if (active !== null && candidates.includes(active)) return active
  return candidates.find((id) => !seen.includes(id)) ?? null
}
