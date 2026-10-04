import { useLocation, useNavigate } from '@tanstack/react-router'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { BranchSelect } from '../../../features/select-branch'
import { useOpenProject, type ProjectRef } from '../../../entities/project'
import { AggregateBlock } from '../../../widgets/aggregate-block'
import { PipelinesList } from '../../../widgets/pipelines-list'

export function ProjectPage(target: ProjectRef) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const openProject = useOpenProject()
  // имя приходит в state навигации; из записи истории его нет — тогда заголовок это путь
  const name = useLocation({ select: (location) => location.state.name })
  return (
    <Card>
      <div className="flex items-center gap-3">
        <Button type="button" variant="ghost" size="sm" onClick={() => void navigate({ to: '/' })}>
          {t('form.projects.back')}
        </Button>
        <h1 translate="no" className="truncate text-xl font-semibold">
          {name ?? target.project}
        </h1>
      </div>
      <BranchSelect {...target} onCommit={(branch) => void openProject({ ...target, branch: branch || undefined, name, replace: true })} />
      <PipelinesList {...target} />
      <AggregateBlock {...target} />
    </Card>
  )
}
