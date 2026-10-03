import { useCallback, type KeyboardEvent, type MouseEvent } from 'react'
import { useReportView } from '../../../entities/report'

const ROW_SELECTOR = '[data-row]'
export const rowAttrs = { 'data-row': '', tabIndex: 0 }

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
    (e: KeyboardEvent<HTMLElement>, id: string): boolean => {
      if (e.key === 'Enter') {
        dispatch({ type: 'select', id })
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
