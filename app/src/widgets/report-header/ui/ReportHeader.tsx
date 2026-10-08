import { useEffect, useMemo, type ReactNode } from 'react'
import { Trans } from 'react-i18next'
import { isLeaf, useReportView } from '../../../entities/report'
import { KeysHelp } from '../../../features/keys-help'
import { Tip } from '../../../features/onboarding'
import { BackToAggregate } from '../../../features/tree-switch'
import { isAggNode, type ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { duration } from '../../../shared/lib/format'
import { Badge } from '../../../shared/ui/badge'
import { LanguageSwitch } from './LanguageSwitch'

const BOLD = { b: <b /> }

const Chip = ({ variant = 'chip', children }: { variant?: 'chip' | 'chipCrit' | 'chipBad'; children: ReactNode }) => (
  <Badge variant={variant} className="px-2 py-0.5 text-[12.5px]">
    {children}
  </Badge>
)

function RootChips({ root }: { root: ReportNode }) {
  const { report } = useReportView()
  if (isAggNode(root)) {
    const { total, end } = root.stats
    const counts = Object.entries(report.meta.statusCounts ?? {}).map(([status, n]) => `${n} ${status.toLowerCase()}`)
    return (
      <>
        <Chip>
          <Trans i18nKey="report.chip.pipelines" count={total} components={BOLD} />
          {counts.length > 0 && ` (${counts.join(', ')})`}
        </Chip>
        <Chip>
          <Trans i18nKey="report.chip.lengthP" values={{ p50: duration(end.p50), p90: duration(end.p90) }} components={BOLD} />
        </Chip>
      </>
    )
  }
  return (
    <>
      {root.ref && <Chip>{root.ref}</Chip>}
      {root.status && <Chip>{root.status}</Chip>}
      <Chip>
        <Trans i18nKey="report.chip.length" values={{ value: duration(root.end) }} components={BOLD} />
      </Chip>
    </>
  )
}

/** Заголовок, чипы пайплайна или агрегата, «← к агрегату», язык и подсказка по управлению. */
export function ReportHeader() {
  const { report, tree, embedded } = useReportView()
  const { t } = useTranslation()
  const { meta } = report
  const root = tree.nodes[tree.root]
  // шапка перерисовывается на каждый кадр перетаскивания, а узлов сотни
  const jobs = useMemo(() => Object.values(tree.nodes).filter(isLeaf).length, [tree])
  const subject = isAggNode(root) ? (meta.label ?? t('report.allPipelines')) : root.name

  useEffect(() => {
    if (embedded) return
    document.title = `Longpole · ${meta.project} · ${subject}`
  }, [embedded, meta.project, subject])

  return (
    <header className="relative flex flex-wrap items-center gap-2 pt-3.5 pb-2.5">
      <h1 className="mr-1 text-[15px] font-semibold" translate="no">
        {meta.project} ·{' '}
        {!isAggNode(root) && root.url ? (
          <a href={root.url} target="_blank" rel="noopener" className="no-underline hover:underline">
            {subject}
          </a>
        ) : (
          subject
        )}
      </h1>
      <RootChips root={root} />
      <Chip variant={tree.totalRetryLoss > 0 ? 'chipBad' : 'chip'}>
        <Trans i18nKey="report.chip.retries" values={{ value: duration(tree.totalRetryLoss) }} components={BOLD} />
      </Chip>
      <Chip variant={tree.saving > 0 ? 'chipCrit' : 'chip'}>
        <Trans i18nKey="report.chip.saving" values={{ value: duration(tree.saving) }} components={BOLD} />
      </Chip>
      <Chip>
        <Trans i18nKey="report.chip.jobs" count={jobs} components={BOLD} />
      </Chip>
      <BackToAggregate />
      {meta.needsMissing && (
        <p role="note" className="basis-full text-[12.5px] text-muted-foreground">
          {t('report.needsMissing')}
        </p>
      )}
      <div className="ml-auto flex items-center gap-2">
        {!embedded && <LanguageSwitch />}
        <Tip id="report.keys">
          <span className="inline-flex">
            <KeysHelp />
          </span>
        </Tip>
      </div>
    </header>
  )
}
