import { createContext, use, useCallback, useEffect, useLayoutEffect, useMemo, useReducer, useRef, type Dispatch, type ReactNode } from 'react'
import type { Report, ReportNode, Tree } from '../../../shared/api'
import type { View } from '../../../shared/lib/timeline'
import { criticalOf, relations, type Critical, type Relations } from './selectors'
import { createReducer, fullView, initialRef, openState, treeOf, type ViewAction, type ViewState } from './state'

type ReportView = {
  report: Report
  tree: Tree<ReportNode>
  state: ViewState
  dispatch: Dispatch<ViewAction>
  crit: Critical
  rel: Relations
  fullView: View
  /** актуальная ось, включая ещё не применённую в кадре */
  getView: () => View
  /** применяется не чаще раза в кадр */
  setView: (view: View) => void
}

const ReportViewContext = createContext<ReportView | null>(null)

export function useReportView(): ReportView {
  const value = use(ReportViewContext)
  if (!value) throw new Error('useReportView вне ReportViewProvider')
  return value
}

export function ReportViewProvider({ report, children }: { report: Report; children: ReactNode }) {
  const reducer = useMemo(() => createReducer(report), [report])
  const [state, dispatch] = useReducer(reducer, report, (r) => openState(r, initialRef(r)))

  const viewRef = useRef(state.view)
  const frame = useRef(0)
  // событие читает ось из ref: между dispatch и рендером state.view ещё старый
  useLayoutEffect(() => {
    if (!frame.current) viewRef.current = state.view
  }, [state.view])
  useEffect(() => () => cancelAnimationFrame(frame.current), [])
  const getView = useCallback(() => viewRef.current, [])
  const setView = useCallback((view: View) => {
    viewRef.current = view
    if (frame.current) return
    frame.current = requestAnimationFrame(() => {
      frame.current = 0
      dispatch({ type: 'setView', view: viewRef.current })
    })
  }, [])

  const tree = treeOf(report, state.tree)
  const crit = useMemo(() => criticalOf(tree, state.scope), [tree, state.scope])
  const rel = useMemo(() => relations(tree, state.selected, state.collapsed), [tree, state.selected, state.collapsed])
  const full = useMemo(() => fullView(tree), [tree])

  const value = useMemo<ReportView>(
    () => ({ report, tree, state, dispatch, crit, rel, fullView: full, getView, setView }),
    [report, tree, state, crit, rel, full, getView, setView],
  )
  return <ReportViewContext value={value}>{children}</ReportViewContext>
}
