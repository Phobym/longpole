import type { ReactNode } from 'react'
import { isAggNode, type Excess, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'
import { cn } from '../../../shared/lib/cn'
import { pos, type View } from '../../../shared/lib/timeline'

const BAR = 'absolute min-w-0.5'
const THICK = `${BAR} top-[calc(50%-6px)] h-3 rounded-[3px]`
const THIN = `${BAR} top-[calc(50%-2px)] h-1 rounded-[2px] bg-thin`
const LINE = `${BAR} top-[calc(50%-1px)] h-0.5 rounded-[1px]`
const GAP = `${BAR} top-[calc(50%-1px)] h-0 border-t-2 border-dashed border-crit`
const RETRY_HATCH = 'bg-[repeating-linear-gradient(135deg,var(--color-retry-a)_0_3px,var(--color-retry-b)_3px_6px)]'
const EXCESS_HATCH = 'bg-[repeating-linear-gradient(135deg,var(--color-over-a)_0_3px,var(--color-over-b)_3px_6px)]'
const LABEL = 'absolute top-[calc(50%-8px)] text-[11px] leading-4 whitespace-nowrap text-muted-foreground'

const STATUS_BG: Record<string, string> = {
  success: 'bg-ok',
  running: 'bg-run',
  pending: 'bg-run',
  failed: 'bg-fail',
  canceled: 'bg-idle',
  skipped: 'bg-idle',
  manual: 'bg-idle',
  created: 'bg-idle',
}

type BarProps = { className: string; from: number; to: number; view: View }

function Bar({ className, from, to, view }: BarProps) {
  const left = pos(from, view)
  return <div className={className} style={{ left: `${left}%`, width: `${Math.max(pos(to, view) - left, 0.1)}%` }} />
}

// подпись правее полоски, а у правого края — левее её начала
function Label({ from, to, view, children }: Omit<BarProps, 'className'> & { children: ReactNode }) {
  const style = pos(to, view) > 80 ? { right: `calc(${100 - pos(from, view)}% + 6px)` } : { left: `calc(${pos(to, view)}% + 6px)` }
  return (
    <span className={LABEL} style={style}>
      {children}
    </span>
  )
}

const NotRun = ({ children }: { children: ReactNode }) => <span className={cn(LABEL, 'left-1')}>{children}</span>

const Strong = ({ className, children }: { className: string; children: ReactNode }) => <b className={cn('font-semibold', className)}>{children}</b>

function mainColor(node: ReportNode, critical: boolean): string {
  if (critical) return 'bg-crit'
  if (isAggNode(node)) return 'bg-ok'
  if (node.allowFailure && node.status === 'failed') return 'bg-warn'
  return STATUS_BG[node.status ?? ''] ?? 'bg-idle'
}

type MainBarProps = { node: ReportNode; from: number; to: number; view: View; critical: boolean; excess: Excess | undefined }

// у узкого места стейджа хвост правее конца остальных джоб заштрихован
function MainBar({ node, from, to, view, critical, excess }: MainBarProps) {
  const partial = isAggNode(node) && node.stats.present < node.stats.total
  // строка группы — тонкая полоска, критическая — оранжевая
  const main = node.kind === 'group' ? cn(THIN, critical && 'bg-crit') : cn(THICK, mainColor(node, critical), partial && 'opacity-50')
  if (excess?.id !== node.id) return <Bar className={main} from={from} to={to} view={view} />
  const cut = Math.min(Math.max(excess.peersEnd, from), to)
  const head = cut > from
  return (
    <>
      {head && <Bar className={cn(main, 'rounded-r-none')} from={from} to={cut} view={view} />}
      <Bar className={cn(THICK, EXCESS_HATCH, head && 'rounded-l-none')} from={cut} to={to} view={view} />
    </>
  )
}

function StageMeta({ node, excess, holderName }: { node: ReportNode; excess: Excess | undefined; holderName: string | undefined }) {
  const { t } = useTranslation()
  const held = excess && (
    <em className="font-semibold text-crit-fg not-italic">
      {t('report.holds')} <span translate="no">{holderName}</span> +{duration(excess.excess)}
    </em>
  )
  let lengths: ReactNode = null
  if (isAggNode(node)) {
    const { duration: d, end: e } = node.stats
    lengths = (
      <>
        <span title="p50 · p90">
          {t('report.stageLength')} <Strong className="text-foreground">{duration(d.p50)}</Strong> · {duration(d.p90)}
        </span>
        <span title="p50 · p90">
          {t('report.stageToEnd')} <Strong className="text-foreground">{duration(e.p50)}</Strong> · {duration(e.p90)}
        </span>
      </>
    )
  } else if (node.start != null && node.end != null) {
    lengths = (
      <>
        <span>
          {t('report.stageLength')} <Strong className="text-foreground">{duration(node.end - node.start)}</Strong>
        </span>
        <span>
          {t('report.stageToEnd')} <Strong className="text-foreground">{duration(node.end)}</Strong>
        </span>
      </>
    )
  }
  // поверх линий связей: подписи не перечёркнуты
  return (
    <div className="absolute inset-y-0 right-3 z-[1] flex items-center gap-3 bg-muted pl-1.5 text-[11px] whitespace-nowrap text-muted-foreground group-data-[selected]:bg-primary">
      {held}
      {lengths}
    </div>
  )
}

type NodeLaneProps = {
  node: ReportNode
  view: View
  critical: boolean
  /** узкое место стейджа, в карточке которого лежит строка */
  excess: Excess | undefined
  /** имя джобы, которая держит стейдж */
  holderName: string | undefined
  /** сколько мс джоба ждала предыдущую на критическом пути */
  gapMs: number | undefined
}

function SingleLane({ node, view, critical, excess, gapMs }: NodeLaneProps & { node: Extract<ReportNode, { start: unknown }> }) {
  const { t } = useTranslation()
  const { start, end } = node
  if (start == null || end == null) return <NotRun>{t('report.notRunStatus', { status: node.status ?? '' })}</NotRun>
  const tries = node.attempts.flatMap((a) => (a.start != null && a.end != null ? [{ start: a.start, end: a.end }] : [])).sort((a, b) => a.start - b.start)
  return (
    <>
      {tries.map((a, i) => {
        const next = tries[i + 1]?.start ?? start
        return (
          <span key={i}>
            <Bar className={cn(THICK, RETRY_HATCH)} from={a.start} to={a.end} view={view} />
            {next > a.end && <Bar className={cn(LINE, 'bg-wait')} from={a.end} to={next} view={view} />}
          </span>
        )
      })}
      {node.queued ? <Bar className={cn(LINE, 'bg-queued')} from={start - node.queued} to={start} view={view} /> : null}
      {gapMs ? <Bar className={GAP} from={start - gapMs} to={start} view={view} /> : null}
      <MainBar node={node} from={start} to={end} view={view} critical={critical} excess={excess} />
      <Label from={start} to={end} view={view}>
        {duration(end - start)}
        {node.attempts.length > 0 && (
          <>
            {' · '}
            <Strong className="text-retry-fg">
              ↻{node.attempts.length}
              {node.retryLoss ? ` −${duration(node.retryLoss)}` : ''}
            </Strong>
          </>
        )}
        {excess?.id === node.id && (
          <>
            {' '}
            <Strong className="text-crit-fg">+{duration(excess.excess)}</Strong>
          </>
        )}
      </Label>
    </>
  )
}

function AggLane({ node, view, critical, excess }: NodeLaneProps & { node: Extract<ReportNode, { stats: unknown }> }) {
  const { t } = useTranslation()
  const st = node.stats
  if (!st.present) return <NotRun>{t('report.notRun')}</NotRun>
  const from = st.start.p50 ?? 0
  const to = from + (st.duration.p50 ?? 0)
  const slow = from + (st.duration.p90 ?? 0)
  return (
    <>
      <Bar className={cn(LINE, 'bg-thin')} from={to} to={slow} view={view} />
      <MainBar node={node} from={from} to={to} view={view} critical={critical} excess={excess} />
      <Label from={from} to={slow} view={view}>
        {duration(st.duration.p50)}
        {st.retried > 0 && (
          <>
            {' · '}
            <Strong className="text-retry-fg">
              ↻ {st.retried}/{st.present}
            </Strong>
          </>
        )}
        {excess?.id === node.id && (
          <>
            {' '}
            <Strong className="text-crit-fg">+{duration(excess.excess)}</Strong>
          </>
        )}
      </Label>
    </>
  )
}

/** Содержимое дорожки строки: полоски, подписи, пунктир превышения. */
export function NodeLane(props: NodeLaneProps) {
  const { node, view, excess } = props
  if (node.kind === 'stage') return <StageMeta node={node} excess={excess} holderName={props.holderName} />
  if (node.kind === 'pipeline') {
    // длина пайплайна уже в шапке и в заголовке bridge
    if (isAggNode(node) || node.start == null || node.end == null) return null
    return <Bar className={THIN} from={node.start} to={node.end} view={view} />
  }
  return (
    <>
      {excess && <div className="absolute inset-y-0 border-l border-dashed border-thin" style={{ left: `${pos(excess.peersEnd, view)}%` }} />}
      {isAggNode(node) ? <AggLane {...props} node={node} /> : <SingleLane {...props} node={node} />}
    </>
  )
}
