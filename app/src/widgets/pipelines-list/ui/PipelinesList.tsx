import { type Pipeline } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { PagedList } from '../../../shared/ui/paged-list'
import { Giraffe } from '../../../shared/ui/giraffe'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { PipelineRow, usePipelines } from '../../../entities/pipeline'
import type { ProjectRef } from '../../../entities/project'

// Своя сборка на строку: у каждой свой Channel, прогресс и ошибка остаются в её строке.
function BuildRow({ pipeline }: { pipeline: Pipeline }) {
  const errorText = useErrorText()
  const { start, busy, progressLabel, error } = useBuild()
  return (
    <PipelineRow
      pipeline={pipeline}
      busy={busy}
      progress={progressLabel}
      error={error && errorText(error)}
      onOpen={() => start(linkForm(pipeline.url))}
    />
  )
}

export function PipelinesList({ host, project, branch, workflow }: ProjectRef) {
  const { t } = useTranslation()
  const query = usePipelines(host, project, branch, workflow)
  const empty = (
    <p aria-live="polite" className="flex items-center gap-3 text-sm text-muted-foreground">
      <Giraffe size={72} />
      {t('form.nothing')}
    </p>
  )
  return (
    <PagedList query={query} empty={empty}>
      {(pipeline) => <BuildRow key={pipeline.id} pipeline={pipeline} />}
    </PagedList>
  )
}
