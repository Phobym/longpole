import { pipelineOf, useReportView } from '../../../entities/report'
import { OpenTree } from '../../../features/tree-switch'
import type { AggNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { duration } from '../../../shared/lib/format'
import { tickStep } from '../../../shared/lib/timeline'
import { Section } from './Section'

const MAX_TICKS = 6
const sampleMs = (x: { start: number; end: number }) => x.end - x.start

/** Длительность джобы по пайплайнам агрегата: точка — один пайплайн, красные — с ретраями. */
export function DurationStrip({ node }: { node: AggNode }) {
  const { report } = useReportView()
  const { t } = useTranslation()
  const { samples, duration: d } = node.stats
  if (!samples.length) return null

  const all = samples.map(sampleMs)
  let lo = Math.min(...all)
  let hi = Math.max(...all)
  if (hi === lo) {
    lo = Math.max(0, lo - 1000)
    hi += 1000
  }
  const at = (ms: number) => `${((ms - lo) / (hi - lo)) * 100}%`
  const step = tickStep(hi - lo, MAX_TICKS)
  const ticks = []
  for (let tick = Math.ceil(lo / step) * step; tick <= hi; tick += step) ticks.push(tick)

  return (
    <Section title={t('report.strip.title', { count: samples.length })}>
      <div className="relative mx-4 h-[50px]">
        <div className="absolute inset-x-0 top-6 border-t" />
        {(['p50', 'p90'] as const).map((kind) => (
          <Mark key={kind} kind={kind} left={at(d[kind] ?? 0)} />
        ))}
        {samples.map((x) => {
          const label = `${pipelineOf(report, x.tree).name} ${duration(sampleMs(x))}`
          return (
            <OpenTree
              key={x.tree}
              index={x.tree}
              aria-label={label}
              title={x.retries ? t('report.strip.dotRetried', { label }) : label}
              className={cn(
                'absolute top-[19px] -ml-[4.5px] size-[9px] cursor-pointer rounded-full hover:opacity-100 focus-visible:opacity-100',
                x.retries ? 'bg-fail opacity-90' : 'bg-ok opacity-55',
              )}
              style={{ left: at(sampleMs(x)) }}
            />
          )
        })}
        {ticks.map((tick) => (
          <span key={tick} className="absolute top-[34px] -translate-x-1/2 text-[10px] whitespace-nowrap text-muted-foreground" style={{ left: at(tick) }}>
            {duration(tick)}
          </span>
        ))}
      </div>
    </Section>
  )
}

// отметки p50 и p90: черта и подпись слева или справа от неё
function Mark({ kind, left }: { kind: 'p50' | 'p90'; left: string }) {
  const p90 = kind === 'p90'
  return (
    <>
      <span className={cn('pointer-events-none absolute top-2.5 h-6 border-l-[1.5px] border-foreground', p90 && 'border-dashed')} style={{ left }} />
      <span className={cn('absolute -top-0.5 text-[10px] whitespace-nowrap', p90 ? 'translate-x-[3px]' : '-translate-x-[calc(100%+3px)]')} style={{ left }}>
        {kind}
      </span>
    </>
  )
}
