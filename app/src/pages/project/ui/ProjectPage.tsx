import { useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { Card } from '../../../shared/ui/card'
import { BranchSelect } from '../../../features/select-branch'
import { useOpenProject, type ProjectRef } from '../../../entities/project'
import { useUpdateSettings } from '../../../entities/settings'
import { AggregateBlock } from '../../../widgets/aggregate-block'
import { PipelinesList } from '../../../widgets/pipelines-list'

export function ProjectPage(target: ProjectRef) {
  const openProject = useOpenProject()
  const update = useUpdateSettings()
  // имя приходит в state навигации; из записи истории его нет — тогда заголовок это путь
  const name = useLocation({ select: (location) => location.state.name })
  // страница смонтирована с key по проекту: эффект идёт один раз на проект
  useEffect(() => {
    void update({ lastProject: { host: target.host, path: target.project } })
  }, [update, target.host, target.project])
  return (
    <Card>
      <h1 translate="no" className="truncate text-[17px] font-semibold">
        {name ?? target.project}
      </h1>
      <BranchSelect {...target} onCommit={(branch) => void openProject({ ...target, branch: branch || undefined, name, replace: true })} />
      <PipelinesList {...target} />
      <AggregateBlock {...target} />
    </Card>
  )
}
