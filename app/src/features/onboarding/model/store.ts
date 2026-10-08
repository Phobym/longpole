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
export const setPersist = (fn: (patch: SettingsPatch) => void) => {
  persist = fn
}

/** Показанная подсказка сразу просмотрена: быстрое закрытие диалога не даёт повтора. */
function repick() {
  const s = useStore.getState()
  const id = pickTip({ order: TIPS, ready: s.ready, seen: s.seen, enabled: s.enabled, enterShown: s.enterShown, active: s.active })
  if (id === null) return
  const seen = [...s.seen, id]
  const enter = TIPS.find((tip) => tip.id === id)?.kind === 'enter'
  useStore.setState({ active: id, seen, enterShown: s.enterShown || enter })
  persist({ seenTips: seen })
}

export function hydrate(enabled: boolean, seen: readonly string[]) {
  useStore.setState((s) => ({ enabled, seen, active: enabled ? s.active : null }))
  repick()
}

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

export function disableTips() {
  useStore.setState({ enabled: false, active: null })
  persist({ tips: false })
}

/** Новый визит экрана: снова можно одну `enter`. */
export function startVisit() {
  useStore.setState({ enterShown: false })
  repick()
}

export function markLeftReport() {
  useStore.setState({ leftReport: true })
}

export const useTipOpen = (id: TipId) => useStore((s) => s.active === id)
export const useLeftReport = () => useStore((s) => s.leftReport)
