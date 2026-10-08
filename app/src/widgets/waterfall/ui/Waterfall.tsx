import { Fragment, useCallback, useMemo, useRef, type KeyboardEvent } from 'react'
import { cards, isLeaf, useReportView } from '../../../entities/report'
import { useGroupToggle } from '../../../features/group-toggle'
import { useNodeSelection } from '../../../features/node-selection'
import { anchorTip, useActiveTip, useSeenTips, type TipId } from '../../../features/onboarding'
import { useReportKeys, useViewDrag, useWheelZoom } from '../../../features/timeline-navigation'
import { isAggNode, type ReportNode } from '../../../shared/api'
import { Axis } from './Axis'
import { Row } from './Row'
import { StageCard } from './StageCard'

// подсказки, что висят на строках водопада, в порядке реестра
const ROW_TIPS = ['report.critical', 'report.retries', 'report.aggregate'] as const satisfies readonly TipId[]

/** Ось и строки отчёта: стейджи карточками, жесты и клавиши. */
export function Waterfall() {
  const { tree, state, crit, rel } = useReportView()
  const blocks = useMemo(() => cards(tree, state.collapsed), [tree, state.collapsed])

  // Одна строка на подсказку: первая подходящая в порядке отрисовки. Якорь у строки один, поэтому если строка первая сразу для нескольких подсказок, они идут на ней друг за другом: на следующих визитах — по мере просмотра прежних.
  const seen = useSeenTips()
  const active = useActiveTip()
  const rowTips = useMemo(() => {
    const agg = isAggNode(tree.nodes[tree.root])
    const first: Partial<Record<(typeof ROW_TIPS)[number], string>> = {}
    for (const { rows: entries } of blocks) {
      for (const { node } of entries) {
        if (first['report.critical'] === undefined && isLeaf(node) && crit.ids.has(node.id)) first['report.critical'] = node.id
        if (first['report.retries'] === undefined && !isAggNode(node) && node.attempts.length > 0) first['report.retries'] = node.id
        if (first['report.aggregate'] === undefined && agg && node.stability?.present) first['report.aggregate'] = node.id
      }
    }
    // Кандидаты строки — в порядке реестра; открытая подсказка держится, хотя уже попала в seen.
    const candidates = new Map<string, TipId[]>()
    for (const id of ROW_TIPS) {
      const row = first[id]
      if (row !== undefined) candidates.set(row, [...(candidates.get(row) ?? []), id])
    }
    const tips = new Map<string, TipId>()
    for (const [row, ids] of candidates) {
      const tip = anchorTip(ids, seen, active)
      if (tip !== null) tips.set(row, tip)
    }
    return tips
  }, [blocks, tree, crit, seen, active])

  const host = useRef<HTMLElement>(null)

  const { onMouseDown, justDragged } = useViewDrag()
  useWheelZoom(host)
  useReportKeys()
  const selection = useNodeSelection()
  const groups = useGroupToggle()
  // колбэки хуков стабильны, а сами объекты — нет: от них зависит memo у Row
  const { onRowKeyDown: selectKey } = selection
  const { onRowKeyDown: toggleKey } = groups
  const onKeyDown = useCallback((e: KeyboardEvent<HTMLElement>, node: ReportNode) => selectKey(e, node) || toggleKey(e, node), [selectKey, toggleKey])

  const { nodes } = tree
  return (
    <>
      <div className="sticky top-0 z-[2] bg-background pt-1 pb-1.5">
        <Axis view={state.view} />
      </div>
      <main
        ref={host}
        className="relative"
        onMouseDown={onMouseDown}
        // отпускание после перетаскивания не должно менять выделение
        onClickCapture={(e) => justDragged() && e.stopPropagation()}
      >
        {blocks.map((block, i) => {
          const { card, rows: entries } = block
          const excess = card ? nodes[card].excess : undefined
          const holderName = excess && nodes[excess.id]?.name
          const content = entries.map(({ node, depth }) => {
            const dep = rel.up.has(node.id) ? 'up' : rel.down.has(node.id) ? 'down' : null
            return (
              <Row
                key={node.id}
                node={node}
                depth={depth}
                view={state.view}
                selected={state.selected === node.id}
                critical={crit.ids.has(node.id)}
                tip={rowTips.get(node.id) ?? null}
                dep={dep}
                depName={dep ? rel.name : ''}
                collapsed={state.collapsed.has(node.id)}
                excess={excess}
                holderName={holderName}
                afterName={node.after ? nodes[node.after]?.name : undefined}
                gapMs={crit.gapBefore.get(node.id)}
                onClick={selection.onRowClick}
                onKeyDown={onKeyDown}
                onToggle={groups.toggle}
              />
            )
          })
          return <Fragment key={`${i}:${entries[0].node.id}`}>{card ? <StageCard cont={block.cont}>{content}</StageCard> : content}</Fragment>
        })}
      </main>
    </>
  )
}
