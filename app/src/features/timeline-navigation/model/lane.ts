// Разметка дорожки: жесты ищут `TRACK` внутри `LANE` и по его ширине переводят пиксели в миллисекунды.
export const laneAttrs = { 'data-lane': '' }
export const trackAttrs = { 'data-track': '' }
const LANE_SELECTOR = '[data-lane]'
const TRACK_SELECTOR = '[data-track]'

/** Дорожка строки, над которой событие, или `null` (колонка имён, не элемент). */
export const trackOf = (target: EventTarget | null) =>
  target instanceof Element ? (target.closest(LANE_SELECTOR)?.querySelector(TRACK_SELECTOR) ?? null) : null
