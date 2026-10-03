import { useCallback, type KeyboardEvent } from 'react'
import { useReportView } from '../../../entities/report'
import type { ReportNode } from '../../../shared/api'

/** Свёртка и развёртка строк с детьми. */
export function useGroupToggle() {
  const { dispatch } = useReportView()

  const toggle = useCallback((id: string) => dispatch({ type: 'toggle', id }), [dispatch])

  /** `←` свернуть, `→` развернуть; на строке без детей ничего. `true`, если клавиша обработана. */
  const onRowKeyDown = useCallback(
    (e: KeyboardEvent, node: ReportNode): boolean => {
      if ((e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') || !node.children.length) return false
      dispatch({ type: 'toggle', id: node.id, collapse: e.key === 'ArrowLeft' })
      return true
    },
    [dispatch],
  )

  return { toggle, onRowKeyDown }
}
