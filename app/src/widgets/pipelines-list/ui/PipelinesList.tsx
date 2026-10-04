import { type Pipeline } from '../../../shared/api'
import { apiErrorText, useTranslation } from '../../../shared/i18n'
import { PagedList } from '../../../shared/ui/paged-list'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { PipelineRow, pipelineUrl, usePipelines } from '../../../entities/pipeline'

type Target = { host: string; project: string }

// Своя сборка на строку: у каждой свой Channel, прогресс и ошибка остаются в её строке.
function BuildRow({ host, project, pipeline }: Target & { pipeline: Pipeline }) {
  const { t } = useTranslation()
  const { start, progress, error } = useBuild()
  return (
    <PipelineRow
      pipeline={pipeline}
      progress={progress}
      error={error && apiErrorText(t, error)}
      onOpen={() => start(linkForm(pipelineUrl(host, project, pipeline.id)))}
    />
  )
}

export function PipelinesList({ host, project, branch }: Target & { branch: string | undefined }) {
  const query = usePipelines(host, project, branch)
  return <PagedList query={query}>{(pipeline) => <BuildRow key={pipeline.id} host={host} project={project} pipeline={pipeline} />}</PagedList>
}
