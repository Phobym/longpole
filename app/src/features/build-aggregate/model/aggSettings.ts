import { create } from 'zustand'
import type { Form, Status } from '../../../shared/api'

export const STATUSES: readonly Status[] = ['SUCCESS', 'MANUAL', 'FAILED', 'CANCELED', 'RUNNING']
const DEFAULT_STATUSES: readonly Status[] = ['SUCCESS', 'MANUAL']
const DEFAULT_LAST = '50'

type State = {
  /** Текст поля: «пусто» и «2.5» должны дойти до ядра как есть, оно сообщит об ошибке. */
  last: string
  /** `null` — «любой» (без фильтра). */
  statuses: readonly Status[] | null
}

// Живёт между проектами и не сохраняется между запусками: `last` при старте 50. Меняет его только запись истории.
const useAggStore = create<State>(() => ({ last: DEFAULT_LAST, statuses: DEFAULT_STATUSES }))

export const useAggLast = () => useAggStore((s) => s.last)
export const useAggStatuses = () => useAggStore((s) => s.statuses)
export const readAggSettings = (): State => useAggStore.getState()

export const setAggLast = (last: string) => useAggStore.setState({ last })
export const toggleAny = () => useAggStore.setState(({ statuses }) => ({ statuses: statuses === null ? [] : null }))
// порядок как в STATUSES: одинаковый набор — одинаковый запрос, а значит одна запись истории
export const toggleStatus = (status: Status) =>
  useAggStore.setState(({ statuses }) =>
    statuses === null ? {} : { statuses: STATUSES.filter((s) => (s === status ? !statuses.includes(s) : statuses.includes(s))) },
  )

/** Запись истории режима «агрегат»; без статусов (старые записи) — как по умолчанию. */
export const fillAggSettings = (form: Pick<Form, 'last' | 'statuses'>) => {
  const known = STATUSES.filter((s) => form.statuses.includes(s))
  useAggStore.setState({
    last: form.last || DEFAULT_LAST,
    statuses: form.statuses.includes('ANY') ? null : known.length ? known : DEFAULT_STATUSES,
  })
}
