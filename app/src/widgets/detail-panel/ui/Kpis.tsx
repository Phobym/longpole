import { isAggNode, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'

function Kpi({ name, value, small }: { name: string; value: string; small?: string }) {
  return (
    <div className="min-w-0 rounded-lg border bg-card px-2.5 py-2 whitespace-nowrap">
      <small className="block text-[11px] text-muted-foreground">{name}</small>
      <b className="mt-0.5 block text-[17px] font-semibold">{value}</b>
      {small && <i className="text-[11px] text-muted-foreground not-italic">{small}</i>}
    </div>
  )
}

/** «Длительность», «Старт», «До конца»; в агрегате — p50 и строка p90. */
export function Kpis({ node }: { node: ReportNode }) {
  const { t } = useTranslation()
  let kpis
  if (isAggNode(node)) {
    const { present, duration: d, start, end } = node.stats
    if (!present) return null
    const two = (name: 'duration' | 'start' | 'toEnd', p: typeof d) => <Kpi name={t(`report.kpi.${name}`)} value={duration(p.p50)} small={t('report.kpi.p90', { time: duration(p.p90) })} />
    kpis = (
      <>
        {two('duration', d)}
        {two('start', start)}
        {two('toEnd', end)}
      </>
    )
  } else {
    if (node.start == null || node.end == null) return null
    kpis = (
      <>
        <Kpi name={t('report.kpi.duration')} value={duration(node.end - node.start)} />
        <Kpi name={t('report.kpi.start')} value={duration(node.start)} />
        <Kpi name={t('report.kpi.toEnd')} value={duration(node.end)} />
      </>
    )
  }
  return <div className="grid grid-cols-3 gap-2">{kpis}</div>
}
