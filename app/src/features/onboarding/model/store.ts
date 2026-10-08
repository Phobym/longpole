import { create } from 'zustand'
import type { SettingsPatch } from '../../../shared/api'
import { pickTip } from './pick'
import { TIPS, type TipId } from './tips'

type State = {
  /** до `hydrate` выключено: в сохранённом HTML-отчёте настроек нет, и подсказки там не показываются */
  enabled: boolean
  seen: readonly string[]
  ready: readonly TipId[]
  active: TipId | null
  enterShown: boolean
  leftReport: boolean
}

const useStore = create<State>(() => ({ enabled: false, seen: [], ready: [], active: null, enterShown: false, leftReport: false }))

// запись в settings.json ставит `OnboardingSync`: стор лежит ниже react-query и не знает о нём
let persist: (patch: SettingsPatch) => void = () => {}
// записи идут по очереди: `seenTips` заменяет список целиком, и обогнавший старый патч вернул бы показанную подсказку
export const setPersist = (fn: (patch: SettingsPatch) => Promise<unknown> | void) => {
  let queue: Promise<unknown> = Promise.resolve()
  persist = (patch) => {
    queue = queue.then(() => fn(patch))
  }
}

/** Показанная подсказка сразу просмотрена: быстрое закрытие диалога не даёт повтора. */
function repick() {
  const s = useStore.getState()
  const id = pickTip<TipId>({ order: TIPS, ready: s.ready, seen: s.seen, enabled: s.enabled, enterShown: s.enterShown, active: s.active })
  if (id === null) return
  const seen = [...s.seen, id]
  const enter = TIPS.find((tip) => tip.id === id)?.kind === 'enter'
  useStore.setState({ active: id, seen, enterShown: s.enterShown || enter })
  persist({ seenTips: seen })
}

/** Просмотренные только добавляются: устаревший ответ настроек не вернёт показанную подсказку. Сброс — `resetTips`. */
export function hydrate(enabled: boolean, seen: readonly string[]) {
  useStore.setState((s) => ({ enabled, seen: [...new Set([...s.seen, ...seen])], active: enabled ? s.active : null }))
  repick()
}

// у каждой подсказки один якорь: `ready` — множество, не счётчик
export function addReady(id: TipId) {
  useStore.setState((s) => (s.ready.includes(id) ? s : { ready: [...s.ready, id] }))
  repick()
}

/** Активная уходит через микротаск: StrictMode в dev сразу монтирует эффект заново, и подсказка не должна гаснуть. */
export function removeReady(id: TipId) {
  useStore.setState((s) => ({ ready: s.ready.filter((r) => r !== id) }))
  queueMicrotask(() => {
    const s = useStore.getState()
    if (s.active !== id || s.ready.includes(id)) return
    useStore.setState({ active: null })
    repick()
  })
}

export function closeTip() {
  useStore.setState({ active: null })
  repick()
}

/** Переключатель в настройках и «Больше не показывать»: стор — источник правды, в `settings.json` уходит через `persist`. */
export function setTipsEnabled(enabled: boolean) {
  useStore.setState(enabled ? { enabled } : { enabled, active: null })
  persist({ tips: enabled })
  repick()
}

export const disableTips = () => setTipsEnabled(false)

/** «Показать заново» в настройках: подсказки включены, просмотренных нет. */
export function resetTips() {
  useStore.setState({ enabled: true, seen: [], active: null, leftReport: false })
  persist({ tips: true, seenTips: [] })
  repick()
}

/** Новый визит экрана: снова можно одну `enter`. Эффект страницы идёт после эффектов якорей — enter-подсказка этого экрана могла уже открыться. */
export function startVisit() {
  const s = useStore.getState()
  const shownHere = s.active !== null && s.ready.includes(s.active) && TIPS.find((tip) => tip.id === s.active)?.kind === 'enter'
  useStore.setState({ enterShown: shownHere })
  repick()
}

export function markLeftReport() {
  useStore.setState({ leftReport: true })
}

export const useTipOpen = (id: TipId | null) => useStore((s) => id !== null && s.active === id)
export const useSeenTips = () => useStore((s) => s.seen)
export const useActiveTip = () => useStore((s) => s.active)
export const useTipsEnabled = () => useStore((s) => s.enabled)
export const useLeftReport = () => useStore((s) => s.leftReport)
