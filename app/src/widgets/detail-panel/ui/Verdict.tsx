import { Fragment, type ReactNode } from 'react'
import { Trans } from 'react-i18next'
import { CRITICAL_SHARE, useReportView } from '../../../entities/report'
import { isAggNode, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'

const MARKUP = { b: <b />, n: <span translate="no" /> }

/** Вывод по узлу: на критическом пути или нет, экономия, что держит, ретраи. */
export function Verdict({ node }: { node: ReportNode }) {
  const { tree } = useReportView()
  const { t } = useTranslation()
  const agg = isAggNode(node)
  const parts: ReactNode[] = []

  if (node.kind === 'stage') {
    const { excess } = node
    if (excess) parts.push(<Trans i18nKey="report.verdict.holds" values={{ name: tree.nodes[excess.id]?.name, time: duration(excess.excess) }} components={MARKUP} />)
    if (!agg) parts.push(t('report.verdict.stagePath'))
  } else if (node.kind === 'pipeline') {
    if (!agg) parts.push(t('report.verdict.pipelinePath'))
  } else {
    const share = node.critShare ?? 0
    const percent = Math.round(share * 100)
    if (!share) parts.push(<Trans i18nKey="report.verdict.off" components={MARKUP} />)
    else if (!agg) parts.push(<Trans i18nKey="report.verdict.on" components={MARKUP} />)
    else parts.push(<Trans i18nKey={share >= CRITICAL_SHARE ? 'report.verdict.onAgg' : 'report.verdict.onAggSometimes'} values={{ percent }} components={MARKUP} />)
    if (node.saving) parts.push(<Trans i18nKey="report.verdict.saving" values={{ time: duration(node.saving) }} components={MARKUP} />)
    if (node.holds) parts.push(<Trans i18nKey="report.verdict.holdsStage" values={{ stage: tree.nodes[node.holds.stage]?.name, time: duration(node.holds.excess) }} components={MARKUP} />)
    if (agg) {
      if (node.stats.retried) parts.push(<Trans i18nKey="report.verdict.retriesAgg" values={{ retried: node.stats.retried, present: node.stats.present }} components={MARKUP} />)
    } else if (node.attempts.length) {
      parts.push(
        <>
          <Trans i18nKey="report.verdict.attempts" count={node.attempts.length} components={MARKUP} />
          {node.retryLoss ? t('report.verdict.lost', { time: duration(node.retryLoss) }) : ''}.
        </>,
      )
    }
  }

  if (!parts.length) return null
  return (
    <p className="mb-3 rounded-lg border bg-card px-2.5 py-2">
      {parts.map((part, i) => (
        <Fragment key={i}>
          {i > 0 && ' '}
          {part}
        </Fragment>
      ))}
    </p>
  )
}
