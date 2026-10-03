import { useCallback, type KeyboardEvent, type MouseEvent } from 'react'
import { flushSync } from 'react-dom'
import { useReportView } from '../../../entities/report'
import type { ReportNode } from '../../../shared/api'

const ROW_SELECTOR = '[data-row]'
export const rowAttrs = { 'data-row': '', tabIndex: 0 }
const rowById = (id: string) => document.querySelector<HTMLElement>(`${ROW_SELECTOR}[data-id="${CSS.escape(id)}"]`)

/** Выбор строки мышью и клавишами. */
export function useNodeSelection() {
  const { dispatch } = useReportView()

  // mousedown на дорожке отменён ради перетаскивания, поэтому фокус на строку ставится вручную
  const onRowClick = useCallback(
    (e: MouseEvent<HTMLElement>, id: string) => {
      dispatch({ type: 'select', id })
      e.currentTarget.focus({ preventScroll: true })
    },
    [dispatch],
  )

  /** `↑` `↓` — соседняя строка, `Enter` — выбрать; `true`, если клавиша обработана. */
  const onRowKeyDown = useCallback(
    (e: KeyboardEvent<HTMLElement>, node: ReportNode): boolean => {
      if (e.key === 'Enter') {
        dispatch({ type: 'select', id: node.id })
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        const rows = [...document.querySelectorAll<HTMLElement>(ROW_SELECTOR)]
        rows[rows.indexOf(e.currentTarget) + (e.key === 'ArrowDown' ? 1 : -1)]?.focus()
      } else {
        return false
      }
      return true
    },
    [dispatch],
  )

  return { onRowClick, onRowKeyDown }
}

/** Выделить узел из панели или «Куда направить силы»: группы над ним развёрнуты, строка в центре и в фокусе. */
export function useRevealNode() {
  const { dispatch } = useReportView()
  return useCallback(
    (id: string) => {
      // строка появляется в DOM только после рендера, поэтому он синхронный
      flushSync(() => dispatch({ type: 'reveal', id }))
      const row = rowById(id)
      row?.scrollIntoView({ block: 'center' })
      row?.focus({ preventScroll: true })
    },
    [dispatch],
  )
}
