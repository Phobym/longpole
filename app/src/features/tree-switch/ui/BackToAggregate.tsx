import { useReportView } from '../../../entities/report'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'

/** «← к агрегату»: только в пайплайне, открытом из агрегата. */
export function BackToAggregate() {
  const { report, state, dispatch } = useReportView()
  const { t } = useTranslation()
  if (report.mode !== 'aggregate' || state.tree === 'agg') return null
  return (
    <Button variant="outline" className="h-auto rounded-full px-2.5 py-[3px] text-[12.5px] font-normal" onClick={() => dispatch({ type: 'open', tree: 'agg' })}>
      {t('report.back')}
    </Button>
  )
}
