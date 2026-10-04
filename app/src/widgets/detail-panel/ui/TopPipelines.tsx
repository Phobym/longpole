import { useMemo } from 'react'
import { pipelineOf, useReportView } from '../../../entities/report'
import { OpenTree } from '../../../features/tree-switch'
import type { AggNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { duration } from '../../../shared/lib/format'
import { GitlabLink } from './GitlabLink'
import { Section } from './Section'

const TOP = 5

/** «Самые долгие»: пять пайплайнов с этим узлом, «Все N» раскрывает остальные. */
export function TopPipelines({ node }: { node: AggNode }) {
  const { report, state, dispatch } = useReportView()
  const { t } = useTranslation()
  const all = useMemo(() => [...node.stats.samples].sort((a, b) => b.end - b.start - (a.end - a.start)), [node])
  if (!all.length) return null
  const longest = all[0].end - all[0].start || 1
  const expanded = state.showAll

  return (
    <Section title={t('report.panel.longest')}>
      {(expanded ? all : all.slice(0, TOP)).map((x) => {
        const pipeline = pipelineOf(report, x.tree)
        const ms = x.end - x.start
        return (
          <div key={x.tree} className="grid min-h-[22px] grid-cols-[64px_1fr_48px_auto] items-center gap-2">
            <OpenTree index={x.tree} className="cursor-pointer text-left text-primary">
              {pipeline.name}
            </OpenTree>
            <span>
              <span className={cn('block h-1.5 rounded-[3px]', x.retries ? 'bg-fail' : 'bg-ok')} style={{ width: `${(ms / longest) * 100}%` }} />
            </span>
            <span className="text-right">{duration(ms)}</span>
            <GitlabLink url={pipeline.url} className="text-[11px] text-muted-foreground">
              {t('report.panel.gitlab')}
            </GitlabLink>
          </div>
        )
      })}
      {all.length > TOP && (
        <button type="button" className="mt-1.5 cursor-pointer text-primary" aria-expanded={expanded} onClick={() => dispatch({ type: 'toggleShowAll' })}>
          {expanded ? t('report.panel.collapseAll') : t('report.panel.showAll', { count: all.length })}
        </button>
      )}
    </Section>
  )
}
