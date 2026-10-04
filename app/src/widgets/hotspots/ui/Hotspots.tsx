import { Trans } from 'react-i18next'
import { useReportView } from '../../../entities/report'
import { useRevealNode } from '../../../features/node-selection'
import { isAggNode, type Hotspot } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'
import { Card } from '../../../shared/ui/card'

const NAME = { n: <span translate="no" /> }

function Reason({ hotspot }: { hotspot: Hotspot }) {
  const { tree } = useReportView()
  const { t } = useTranslation()
  if (hotspot.kind === 'excess') {
    const key = isAggNode(tree.nodes[tree.root]) ? 'report.hotspots.excess' : 'report.hotspots.excessCritical'
    return <Trans i18nKey={key} values={{ stage: hotspot.stage, excess: duration(hotspot.excess) }} components={NAME} />
  }
  if ('retried' in hotspot) return t('report.hotspots.retriesAgg', { retried: hotspot.retried, present: hotspot.present })
  // в пайплайне, открытом из агрегата, добавлена стабильность по последним запускам
  const stability = tree.nodes[hotspot.id]?.stability
  return (
    <>
      {t('report.hotspots.retries', { count: hotspot.retries })}
      {stability && t('report.hotspots.retriesLast', stability)}
    </>
  )
}

/** «Куда направить силы»: до трёх джоб критического пути, где экономия больше всего. */
export function Hotspots() {
  const { tree } = useReportView()
  const { t } = useTranslation()
  const reveal = useRevealNode()
  if (!tree.hotspots.length) return null
  return (
    <Card className="mt-px mb-2.5 gap-0 overflow-hidden rounded-[10px] border-0 p-0 pb-1.5 shadow-none ring-1 ring-crit-soft">
      <h2 className="m-0 bg-crit-bg px-3 py-[7px] text-[12.5px] font-semibold text-crit-fg">{t('report.hotspots.title')}</h2>
      {tree.hotspots.map((h) => (
        <button key={`${h.id}:${h.kind}`} type="button" className="flex w-full cursor-pointer items-baseline gap-2 px-3 py-1 text-left hover:bg-accent" onClick={() => reveal(h.id)}>
          <span className="min-w-[62px] font-semibold text-crit-fg">−{duration(h.saving)}</span>
          <span>
            <span translate="no">{h.name}</span> <span className="text-muted-foreground">— <Reason hotspot={h} /></span>
          </span>
        </button>
      ))}
    </Card>
  )
}
