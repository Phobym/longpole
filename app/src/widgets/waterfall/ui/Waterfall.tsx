import { Fragment, useCallback, useMemo, useRef, type KeyboardEvent } from 'react'
import { cards, useReportView } from '../../../entities/report'
import { useGroupToggle } from '../../../features/group-toggle'
import { useNodeSelection } from '../../../features/node-selection'
import { useReportKeys, useViewDrag, useWheelZoom } from '../../../features/timeline-navigation'
import type { ReportNode } from '../../../shared/api'
import { Axis } from './Axis'
import { LinksOverlay } from './LinksOverlay'
import { Row } from './Row'
import { StageCard } from './StageCard'

/** Ось и строки отчёта: стейджи карточками, жесты и клавиши, линии связей выбранного узла. */
export function Waterfall() {
  const { tree, state, crit, rel } = useReportView()
  const blocks = useMemo(() => cards(tree, state.collapsed), [tree, state.collapsed])

  const host = useRef<HTMLElement>(null)
  const track = useRef<HTMLDivElement>(null)
  const rows = useRef(new Map<string, HTMLElement>())
  const register = useCallback((id: string, el: HTMLElement | null) => {
    if (el) rows.current.set(id, el)
    else rows.current.delete(id)
  }, [])

  const { onMouseDown, justDragged } = useViewDrag()
  useWheelZoom(host)
  useReportKeys()
  const selection = useNodeSelection()
  const groups = useGroupToggle()
  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLElement>, node: ReportNode) => selection.onRowKeyDown(e, node.id) || groups.onRowKeyDown(e, node),
    [selection, groups],
  )

  const { nodes } = tree
  return (
    <>
      <div className="sticky top-0 z-[2] bg-background pt-1 pb-1.5">
        <Axis view={state.view} trackRef={track} />
      </div>
      <main
        ref={host}
        className="relative"
        onMouseDown={onMouseDown}
        // отпускание после перетаскивания не должно менять выделение
        onClickCapture={(e) => justDragged() && e.stopPropagation()}
      >
        {blocks.map(({ card, cont, rows: entries }, i) => {
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
                dep={dep}
                depName={dep ? rel.name : ''}
                collapsed={state.collapsed.has(node.id)}
                excess={excess}
                holderName={holderName}
                afterName={node.after ? nodes[node.after]?.name : undefined}
                gapMs={crit.gapBefore.get(node.id)}
                register={register}
                onClick={selection.onRowClick}
                onKeyDown={onKeyDown}
                onToggle={groups.toggle}
              />
            )
          })
          return <Fragment key={`${i}:${entries[0].node.id}`}>{card ? <StageCard cont={cont}>{content}</StageCard> : content}</Fragment>
        })}
        <LinksOverlay host={host} track={track} rows={rows} />
      </main>
    </>
  )
}
