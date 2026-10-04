import { useLocation, useNavigate } from '@tanstack/react-router'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { BranchSelect } from '../../../features/select-branch'
import { useOpenProject } from '../../../entities/project'
import { AggregateBlock } from '../../../widgets/aggregate-block'
import { PipelinesList } from '../../../widgets/pipelines-list'

type Props = { host: string; fullPath: string; branch: string | undefined }

export function ProjectPage({ host, fullPath, branch }: Props) {
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
          {name ?? fullPath}
        </h1>
      </div>
      <BranchSelect
        host={host}
        project={fullPath}
        value={branch}
        onCommit={(ref) => void openProject({ host, fullPath, ref: ref || undefined, name, replace: true })}
      />
      <PipelinesList host={host} project={fullPath} branch={branch} />
      <AggregateBlock host={host} project={fullPath} branch={branch} />
    </Card>
  )
}
