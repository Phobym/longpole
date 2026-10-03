import { ReportViewProvider } from '../../../entities/report'
import type { Report } from '../../../shared/api'
import { Waterfall } from '../../../widgets/waterfall'

export function ReportPage({ report }: { report: Report }) {
  return (
    <ReportViewProvider report={report}>
      <div className="px-4 pb-8 text-[12.5px] leading-[1.35]">
        <Waterfall />
      </div>
    </ReportViewProvider>
  )
}
