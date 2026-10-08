import { type Pipeline } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
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
  const query = usePipelines(host, project, branch, workflow)
  return (
    <PagedList query={query} emptyIcon={<Giraffe size={72} />}>
      {(pipeline) => <BuildRow key={pipeline.id} pipeline={pipeline} />}
    </PagedList>
  )
}
