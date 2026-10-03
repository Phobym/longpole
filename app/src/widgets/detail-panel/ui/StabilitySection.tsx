import type { ReactNode } from 'react'
import { isAggNode, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'
import { GitlabLink } from './GitlabLink'
import { Section } from './Section'

/** Попытки, потери и очередь джобы; у остальных узлов раздела нет. */
export function StabilitySection({ node }: { node: ReportNode }) {
  const { t } = useTranslation()
  if (node.kind !== 'job' && node.kind !== 'bridge') return null
  const rows: [string, ReactNode][] = []
  if (isAggNode(node)) {
    const { retried, present, queued } = node.stats
    rows.push([t('report.panel.retries'), t('report.panel.retriesIn', { count: present, retried, present })])
    if (node.retryLoss) rows.push([t('report.panel.loss'), t('report.panel.lossAvg', { time: duration(node.retryLoss) })])
    if (queued.p50 != null) rows.push([t('report.panel.queue'), t('report.panel.queueP', { p50: duration(queued.p50), p90: duration(queued.p90) })])
  } else {
    if (node.attempts.length) {
      const tries = [...node.attempts].sort((a, b) => (a.start ?? 0) - (b.start ?? 0))
      rows.push([
        t('report.panel.attempts'),
        tries.map((a, i) => (
          <div key={i}>
            <GitlabLink url={a.url}>{a.status}</GitlabLink> {a.start == null || a.end == null ? '' : duration(a.end - a.start)}
          </div>
        )),
      ])
    }
    if (node.retryLoss) rows.push([t('report.panel.loss'), duration(node.retryLoss)])
    if (node.queued != null) rows.push([t('report.panel.queue'), duration(node.queued)])
  }
  if (!rows.length) return null
  return (
    <Section title={t('report.panel.stability')}>
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
        {rows.map(([name, value]) => (
          <div key={name} className="contents">
            <dt className="text-muted-foreground">{name}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </Section>
  )
}
