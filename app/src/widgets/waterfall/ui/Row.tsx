import { memo, type KeyboardEvent, type MouseEvent } from 'react'
import { DepTag, NodeLane, StabilityPill, type DepDir } from '../../../entities/node'
import { GroupToggle } from '../../../features/group-toggle'
import { rowAttrs } from '../../../features/node-selection'
import { Tip, type TipId } from '../../../features/onboarding'
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
  /** якорь подсказки онбординга на этой строке; `null` — без подсказки */
  tip: TipId | null
  /** связь с выбранной строкой */
  dep: DepDir | null
  depName: string
  collapsed: boolean
  /** узкое место стейджа, в карточке которого лежит строка */
  excess: Excess | undefined
  holderName: string | undefined
  /** имя соседа, которого джоба ждала внутри стейджа */
  afterName: string | undefined
  gapMs: number | undefined
  // Эти три обязаны быть стабильными, иначе memo бессмысленна.
  onClick: (e: MouseEvent<HTMLElement>, id: string) => void
  onKeyDown: (e: KeyboardEvent<HTMLElement>, node: ReportNode) => void
  onToggle: (id: string) => void
}

/** Строка водопада: имя, дорожка с полосками, метка стабильности. */
export const Row = memo(function Row({ node, depth, view, selected, critical, tip, dep, depName, collapsed, excess, holderName, afterName, gapMs, onClick, onKeyDown, onToggle }: RowProps) {
  const stage = node.kind === 'stage'
  return (
    <Tip id={tip} inset>
      <div
        {...rowAttrs}
        data-id={node.id}
        data-selected={selected ? '' : undefined}
        aria-current={selected || undefined}
        className={cn(
          GRID,
          'group scroll-my-10 cursor-pointer items-stretch focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring',
          stage ? 'h-8 border-b bg-muted' : 'h-[26px]',
          selected
            ? 'bg-primary text-primary-foreground **:text-primary-foreground focus-visible:outline-primary-foreground'
            : stage
              ? 'hover:bg-accent'
              : 'hover:bg-accent odd:not-hover:bg-muted/50',
          dep === 'up' && 'shadow-[inset_3px_0_var(--color-dep-up)]',
          dep === 'down' && 'shadow-[inset_3px_0_var(--color-dep-down)]',
        )}
        onClick={(e) => onClick(e, node.id)}
        onKeyDown={(e) => {
          // клавиши внутри строки (стрелка-переключатель) обрабатывает сам элемент; с ⌘/Ctrl/Alt отчёт клавиши не трогает
          if (e.target === e.currentTarget && !e.ctrlKey && !e.metaKey && !e.altKey && onKeyDown(e, node)) e.preventDefault()
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
    </Tip>
  )
})
