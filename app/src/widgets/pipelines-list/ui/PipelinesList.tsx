import { type Pipeline } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
import { PagedList } from '../../../shared/ui/paged-list'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { PipelineRow, pipelineUrl, usePipelines } from '../../../entities/pipeline'
import type { ProjectRef } from '../../../entities/project'

// Своя сборка на строку: у каждой свой Channel, прогресс и ошибка остаются в её строке.
function BuildRow({ host, project, pipeline }: Pick<ProjectRef, 'host' | 'project'> & { pipeline: Pipeline }) {
  const errorText = useErrorText()
  const { start, busy, progressLabel, error } = useBuild()
  return (
    <PipelineRow
      pipeline={pipeline}
      busy={busy}
      progress={progressLabel}
      error={error && errorText(error)}
      onOpen={() => start(linkForm(pipelineUrl(host, project, pipeline.id)))}
    />
  )
}

export function PipelinesList({ host, project, branch }: ProjectRef) {
  const query = usePipelines(host, project, branch)
  return <PagedList query={query}>{(pipeline) => <BuildRow key={pipeline.id} host={host} project={project} pipeline={pipeline} />}</PagedList>
}
