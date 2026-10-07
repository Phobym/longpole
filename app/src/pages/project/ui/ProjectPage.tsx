import { useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { apiError } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
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
  const errorText = useErrorText()
  // имя приходит в state навигации; из записи истории его нет — тогда заголовок это путь
  const name = useLocation({ select: (location) => location.state.name })
  // страница смонтирована с key по проекту: эффект идёт один раз на проект
  useEffect(() => {
    void update({ lastProject: { host: target.host, path: target.project } })
  }, [update, target.host, target.project])
  // GitHub без выбора (или с устаревшим) — первый workflow; у GitLab список пуст и workflow нет
  const query = useWorkflows(target.host, target.project)
  const workflows = query.data ?? []
  const current: ProjectRef = { ...target, workflow: workflows.find((w) => w.file === target.workflow)?.file ?? workflows[0]?.file }
  const failure = apiError(query.error)
  return (
    <Card>
      <h1 translate="no" className="truncate text-[17px] font-semibold">
        {name ?? target.project}
      </h1>
      <BranchSelect {...current} onCommit={(branch) => void openProject({ ...current, branch: branch || undefined, name, replace: true })} />
      {workflows.length > 0 && current.workflow && (
        <WorkflowSelect workflows={workflows} value={current.workflow} onChange={(workflow) => void openProject({ ...current, workflow, name, replace: true })} />
      )}
      {query.isError && (
        <p role="alert" className="text-sm text-destructive">
          {failure && errorText(failure)}
        </p>
      )}
      {/* пока список workflow не ответил (или ответил ошибкой), запросы пайплайнов и агрегат ушли бы без workflow */}
      {query.isSuccess && (
        <>
          <PipelinesList {...current} />
          <AggregateBlock {...current} />
        </>
      )}
    </Card>
  )
}
