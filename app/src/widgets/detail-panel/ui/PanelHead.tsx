import { Fragment } from 'react'
import { CRITICAL_SHARE, isLeaf, isLinkable, stageOf, useReportView } from '../../../entities/report'
import { StabilityPill } from '../../../entities/node'
import { isAggNode, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { Badge } from '../../../shared/ui/badge'

const CHIP = 'px-2 py-0 text-[10.5px] leading-4'

/** Имя, «стейдж <S>», чипы: стабильность, статус, критический путь. */
export function PanelHead({ node }: { node: ReportNode }) {
  const { tree } = useReportView()
  const { t } = useTranslation()
  const stage = stageOf(tree, node.id)
  const agg = isAggNode(node)
  const share = node.critShare ?? 0
  const critical = agg ? share >= CRITICAL_SHARE : share > 0

  const sub = [
    stage && (
      <>
        {t('report.panel.stage')} <span translate="no">{stage.name}</span>
      </>
    ),
    agg && isLinkable(node) && t('report.panel.ran', { present: node.stats.present, total: node.stats.total }),
  ].filter(Boolean)

  return (
    <>
      <h2 id="panel-title" className="mt-1 mr-7 text-[15px] font-semibold [overflow-wrap:anywhere]" translate="no">
        {node.name}
      </h2>
      {sub.length > 0 && (
        <div className="mt-0.5 text-[11.5px] text-muted-foreground">
          {sub.map((part, i) => (
            <Fragment key={i}>
              {i > 0 && ' · '}
              {part}
            </Fragment>
          ))}
        </div>
      )}
      <div className="my-2.5 mb-3 flex flex-wrap gap-1.5 empty:hidden">
        <StabilityPill stability={node.stability} />
        {!agg && isLeaf(node) && node.status && (
          <Badge variant={node.status === 'failed' && !node.allowFailure ? 'chipBad' : 'chip'} className={CHIP}>
            {node.status}
            {node.allowFailure && ' · allow_failure'}
          </Badge>
        )}
        {isLinkable(node) && (
          <Badge variant={critical ? 'chipCrit' : 'chip'} className={CHIP}>
            {agg ? t('report.panel.critAgg', { percent: Math.round(share * 100) }) : t(share ? 'report.panel.onCritical' : 'report.panel.notOnCritical')}
          </Badge>
        )}
      </div>
    </>
  )
}
