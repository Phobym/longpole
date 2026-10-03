import { memo, type KeyboardEvent, type MouseEvent } from 'react'
import { DepTag, NodeLane, StabilityPill } from '../../../entities/node'
import { GroupToggle } from '../../../features/group-toggle'
import { rowAttrs } from '../../../features/node-selection'
import { laneAttrs, trackAttrs } from '../../../features/timeline-navigation'
import type { Excess, ReportNode } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import type { View } from '../../../shared/lib/timeline'
import { GRID } from './grid'

type RowProps = {
  node: ReportNode
  depth: number
  view: View
  selected: boolean
  critical: boolean
  /** связь с выбранной строкой */
  dep: 'up' | 'down' | null
  depName: string
  collapsed: boolean
  /** узкое место стейджа, в карточке которого лежит строка */
  excess: Excess | undefined
  holderName: string | undefined
  /** имя соседа, которого джоба ждала внутри стейджа */
  afterName: string | undefined
  gapMs: number | undefined
  // Эти четыре обязаны быть стабильными, иначе memo бессмысленна.
  register: (id: string, el: HTMLElement | null) => void
  onClick: (e: MouseEvent<HTMLElement>, id: string) => void
  onKeyDown: (e: KeyboardEvent<HTMLElement>, node: ReportNode) => void
  onToggle: (id: string) => void
}

/** Строка водопада: имя, дорожка с полосками, метка стабильности. */
export const Row = memo(function Row({ node, depth, view, selected, critical, dep, depName, collapsed, excess, holderName, afterName, gapMs, register, onClick, onKeyDown, onToggle }: RowProps) {
  const stage = node.kind === 'stage'
  return (
    <div
      {...rowAttrs}
      ref={(el) => {
        register(node.id, el)
        return () => register(node.id, null)
      }}
      data-id={node.id}
      data-selected={selected ? '' : undefined}
      className={cn(
        GRID,
        'group scroll-my-10 cursor-pointer items-stretch focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring',
        stage ? 'h-8 border-b border-secondary' : 'h-[26px]',
        selected ? 'bg-selected' : 'hover:bg-accent',
        dep === 'up' && 'shadow-[inset_3px_0_var(--color-dep-up)]',
        dep === 'down' && 'shadow-[inset_3px_0_var(--color-dep-down)]',
      )}
      onClick={(e) => onClick(e, node.id)}
      onKeyDown={(e) => {
        // клавиши внутри строки (стрелка-переключатель) обрабатывает сам элемент
        if (e.target === e.currentTarget && onKeyDown(e, node)) e.preventDefault()
      }}
    >
      <div className="flex min-w-0 items-center pr-2 whitespace-nowrap" style={{ paddingLeft: 8 + depth * 12 }} title={node.name}>
        <GroupToggle node={node} collapsed={collapsed} onToggle={onToggle} />
        <span className={cn('overflow-hidden text-ellipsis', stage ? 'text-[13px] font-semibold' : (node.kind === 'pipeline' || critical) && 'font-semibold')} translate="no">
          {node.name}
        </span>
        {afterName && (
          <span className="ml-1.5 shrink overflow-hidden text-[11px] text-ellipsis text-muted-foreground" translate="no">
            ← {afterName}
          </span>
        )}
      </div>
      <div {...laneAttrs} className="relative cursor-grab overflow-hidden">
        <div {...trackAttrs} className="absolute inset-y-0 right-6 left-3">
          <NodeLane node={node} view={view} critical={critical} excess={excess} holderName={holderName} gapMs={gapMs} />
        </div>
        {dep && <DepTag dir={dep} name={depName} />}
      </div>
      <div className="flex items-center justify-end pr-2.5">
        <StabilityPill stability={node.stability} />
      </div>
    </div>
  )
})
