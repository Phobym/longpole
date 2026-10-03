import type { ReportNode, Tree } from '../../../shared/api'
import type { ViewState } from './state'

type Nodes = Tree<ReportNode>['nodes']

const LINKABLE = new Set(['job', 'bridge', 'group'])

/** У этих узлов есть связи `deps`/`dependents`, их линии и метки рисует водопад. */
export const isLinkable = (node: ReportNode) => LINKABLE.has(node.kind)

const leafIds = (nodes: Nodes, node: ReportNode): string[] =>
  node.kind === 'group' ? node.children.flatMap((id) => leafIds(nodes, nodes[id])) : [node.id]

/** Связанный узел внутри свёрнутой группы подсвечивается строкой группы. */
export function visibleId(nodes: Nodes, collapsed: ReadonlySet<string>, id: string): string {
  let visible = id
  for (let p = nodes[id]?.parent; p; p = nodes[p]?.parent) if (collapsed.has(p)) visible = p
  return visible
}

export type Relations = {
  /** от кого выбранный ждал */
  up: ReadonlySet<string>
  /** кто ждёт выбранный */
  down: ReadonlySet<string>
  /** выбранный узел и его шарды: линии к ним считаются «своими» */
  own: ReadonlySet<string>
  name: string
}

const NO_RELATIONS: Relations = { up: new Set(), down: new Set(), own: new Set(), name: '' }

/** Связи выбранного узла; у стейджа и пайплайна их нет. */
export function relations(tree: Tree<ReportNode>, { selected, collapsed }: ViewState): Relations {
  const sel = selected ? tree.nodes[selected] : undefined
  if (!sel || !isLinkable(sel)) return NO_RELATIONS
  const visible = (id: string) => visibleId(tree.nodes, collapsed, id)
  const up = new Set(sel.deps.map(visible))
  const down = new Set((sel.dependents ?? []).map(visible))
  up.delete(sel.id)
  down.delete(sel.id)
  return { up, down, own: new Set([sel.id, ...leafIds(tree.nodes, sel)]), name: sel.name }
}

export type Critical = {
  ids: ReadonlySet<string>
  /** джоба → сколько мс ждала предыдущую на пути */
  gapBefore: ReadonlyMap<string, number>
}

export function criticalOf(tree: Tree<ReportNode>, scope: string): Critical {
  const path = tree.critical[scope]
  return { ids: new Set(path?.ids), gapBefore: new Map(path?.gaps.map((g) => [g.to, g.ms])) }
}

export type RowEntry = { node: ReportNode; depth: number }

/** Подряд идущие строки одного стейджа; `card: null` — строка пайплайна вне карточки. */
export type CardBlock = { card: string | null; cont: boolean; rows: RowEntry[] }

/**
 * Строки водопада по карточкам. Строки downstream-пайплайна внутри bridge закрывают
 * карточку, остаток стейджа продолжается карточкой без заголовка (`cont`).
 */
export function cards(tree: Tree<ReportNode>, collapsed: ReadonlySet<string>): CardBlock[] {
  const blocks: CardBlock[] = []
  const walk = (node: ReportNode, depth: number, card: string | null) => {
    const own = node.kind === 'stage' ? node.id : node.kind === 'pipeline' ? null : card
    let block = blocks.at(-1)
    if (!block || block.card !== own) blocks.push((block = { card: own, cont: node.kind !== 'stage', rows: [] }))
    block.rows.push({ node, depth })
    if (!collapsed.has(node.id)) for (const id of node.children) walk(tree.nodes[id], depth + 1, own)
  }
  walk(tree.nodes[tree.root], 0, null)
  return blocks
}
