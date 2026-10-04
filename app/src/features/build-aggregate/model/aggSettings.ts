import { create } from 'zustand'
import { type Form } from '../../../shared/api'

export const STATUSES = ['SUCCESS', 'MANUAL', 'FAILED', 'CANCELED', 'RUNNING'] as const
const DEFAULT_STATUSES = ['SUCCESS', 'MANUAL']
export const ANY = 'ANY'

type State = {
  /** Текст поля: «пусто» и «2.5» должны дойти до ядра как есть, оно сообщит об ошибке. */
  last: string
  /** `['ANY']` — без фильтра. */
  statuses: string[]
  setLast: (last: string) => void
  toggleStatus: (status: string) => void
  toggleAny: () => void
  fill: (settings: Pick<State, 'last' | 'statuses'>) => void
}

// Живёт между проектами и не сохраняется между запусками: `last` при старте 50. Меняет его только запись истории.
const useAggStore = create<State>((set) => ({
  last: '50',
  statuses: DEFAULT_STATUSES,
  setLast: (last) => set({ last }),
  // порядок как в STATUSES: одинаковый набор — одинаковый запрос, а значит одна запись истории
  toggleStatus: (status) =>
    set(({ statuses }) => ({ statuses: STATUSES.filter((s) => (s === status ? !statuses.includes(s) : statuses.includes(s))) })),
  toggleAny: () => set(({ statuses }) => ({ statuses: statuses.includes(ANY) ? [] : [ANY] })),
  fill: (settings) => set(settings),
}))

/** Запись истории режима «агрегат» без статусов (старые записи) — как по умолчанию. */
export const fillAggSettings = (form: Pick<Form, 'last' | 'statuses'>) =>
  useAggStore.getState().fill({ last: form.last || '50', statuses: form.statuses.length ? form.statuses : DEFAULT_STATUSES })

export const useAggSettings = () => useAggStore()
