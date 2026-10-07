import { useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { Card } from '../../../shared/ui/card'
import { BranchSelect } from '../../../features/select-branch'
import { WorkflowSelect } from '../../../features/select-workflow'
import { useOpenProject, useWorkflows, type ProjectRef } from '../../../entities/project'
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
  // GitHub без выбора — первый workflow; у GitLab список пуст и workflow нет
  const { data: workflows = [] } = useWorkflows(target.host, target.project)
  const current: ProjectRef = { ...target, workflow: target.workflow ?? workflows[0]?.file }
  return (
    <Card>
      <h1 translate="no" className="truncate text-[17px] font-semibold">
        {name ?? target.project}
      </h1>
      <BranchSelect {...current} onCommit={(branch) => void openProject({ ...current, branch: branch || undefined, name, replace: true })} />
      {workflows.length > 0 && current.workflow && (
        <WorkflowSelect workflows={workflows} value={current.workflow} onChange={(workflow) => void openProject({ ...current, workflow, name, replace: true })} />
      )}
      <PipelinesList {...current} />
      <AggregateBlock {...current} />
    </Card>
  )
}
