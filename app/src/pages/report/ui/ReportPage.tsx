import { ReportViewProvider, useReportView } from '../../../entities/report'
import type { Report } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { DetailPanel } from '../../../widgets/detail-panel'
import { Hotspots } from '../../../widgets/hotspots'
import { ReportHeader } from '../../../widgets/report-header'
import { Waterfall } from '../../../widgets/waterfall'

// панель — fixed: на широком окне страница уступает ей место справа
function Layout() {
  const { state } = useReportView()
  return (
    <div className="text-[12.5px] leading-[1.35]">
      <div className={cn('px-4 pb-8', state.selected && 'min-[900px]:mr-[420px]')}>
        <ReportHeader />
        <Hotspots />
        <Waterfall />
      </div>
      <DetailPanel />
    </div>
  )
}

export function ReportPage({ report, embedded }: { report: Report; embedded?: boolean }) {
  return (
    <ReportViewProvider report={report} embedded={embedded}>
      <Layout />
    </ReportViewProvider>
  )
}
