import { isLeaf, useReportView } from '../../../entities/report'
import { isAggNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { DurationStrip } from './DurationStrip'
import { GitlabLink } from './GitlabLink'
import { Kpis } from './Kpis'
import { PanelHead } from './PanelHead'
import { Relations } from './Relations'
import { StabilitySection } from './StabilitySection'
import { TopPipelines } from './TopPipelines'
import { Verdict } from './Verdict'

/** Панель справа: сужает водопад на широком окне, на узком перекрывает его. */
export function DetailPanel() {
  const { tree, state, dispatch } = useReportView()
  const { t } = useTranslation()
  const node = state.selected ? tree.nodes[state.selected] : undefined
  if (!node) return null
  const agg = isAggNode(node)
  return (
    <aside aria-labelledby="panel-title" className="fixed inset-y-0 right-0 z-[3] w-[min(420px,100vw)] overflow-x-hidden overflow-y-auto overscroll-contain border-l bg-chrome px-[18px] pt-3.5 pb-6">
      <Button variant="ghost" className="absolute top-2.5 right-3 size-[26px] text-base text-muted-foreground" aria-label={t('report.panel.close')} onClick={() => dispatch({ type: 'close' })}>
        ×
      </Button>
      <PanelHead node={node} />
      <Verdict node={node} />
      <Kpis node={node} />
      {agg && isLeaf(node) && <DurationStrip node={node} />}
      <StabilitySection node={node} />
      <Relations />
      {agg ? <TopPipelines node={node} /> : node.url && (
        <p className="mt-4">
          <GitlabLink url={node.url}>{t('report.panel.openGitlab')}</GitlabLink>
        </p>
      )}
    </aside>
  )
}
