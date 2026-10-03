import { isAggNode, type Report, type ReportNode, type Tree } from '../../../shared/api'
import type { View } from '../../../shared/lib/timeline'

/** Какое дерево открыто: агрегат или пайплайн по индексу в `trees`. */
export type TreeRef = 'agg' | number

export type ViewState = {
  tree: TreeRef
  collapsed: ReadonlySet<string>
  selected: string | null
  /** узел, до конца которого считается критический путь (в агрегате — всегда корень) */
  scope: string
  /** «Самые долгие» раскрыты целиком */
  showAll: boolean
  view: View
}

export type ViewAction =
  | { type: 'open'; tree: TreeRef }
  | { type: 'select'; id: string }
  | { type: 'toggle'; id: string; collapse?: boolean }
  /** выделить узел и развернуть группы над ним */
  | { type: 'reveal'; id: string }
  | { type: 'setView'; view: View }
  | { type: 'close' }
  | { type: 'toggleShowAll' }

export const treeOf = (report: Report, ref: TreeRef): Tree<ReportNode> =>
  report.mode === 'single' ? report.tree : ref === 'agg' ? report.agg : report.trees[ref]

/** Видно весь пайплайн; в агрегате — до p90 конца. */
export function fullView(tree: Tree<ReportNode>): View {
  const root = tree.nodes[tree.root]
  return [0, (isAggNode(root) ? root.stats.end.p90 : root.end) || 1]
}

/** Все группы свёрнуты, выделения нет, ось на весь пайплайн. */
export function openState(report: Report, ref: TreeRef): ViewState {
  const tree = treeOf(report, ref)
  return {
    tree: ref,
    collapsed: new Set(Object.values(tree.nodes).filter((n) => n.kind === 'group').map((n) => n.id)),
    selected: null,
    scope: tree.root,
    showAll: false,
    view: fullView(tree),
  }
}

function selectNode(state: ViewState, tree: Tree<ReportNode>, id: string): ViewState {
  const node = tree.nodes[id]
  if (!node) return state
  // в одиночном пайплайне выбор стейджа или пайплайна переносит критический путь
  const moveScope = state.tree !== 'agg' && (node.kind === 'stage' || node.kind === 'pipeline')
  return {
    ...state,
    selected: id,
    scope: moveScope ? id : state.scope,
    showAll: state.selected === id && state.showAll,
  }
}

export const createReducer = (report: Report) =>
  function reduce(state: ViewState, action: ViewAction): ViewState {
    const tree = treeOf(report, state.tree)
    switch (action.type) {
      case 'open':
        return openState(report, action.tree)
      case 'select':
        return selectNode(state, tree, action.id)
      case 'toggle': {
        const collapse = action.collapse ?? !state.collapsed.has(action.id)
        const collapsed = new Set(state.collapsed)
        if (collapse) collapsed.add(action.id)
        else collapsed.delete(action.id)
        return { ...state, collapsed }
      }
      case 'reveal': {
        const collapsed = new Set(state.collapsed)
        for (let p = tree.nodes[action.id]?.parent; p; p = tree.nodes[p]?.parent) collapsed.delete(p)
        return selectNode({ ...state, collapsed }, tree, action.id)
      }
      case 'setView':
        return { ...state, view: action.view }
      case 'close':
        return { ...state, selected: null }
      case 'toggleShowAll':
        return { ...state, showAll: !state.showAll }
    }
  }
