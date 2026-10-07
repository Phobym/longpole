import { aggregateForm, useBuild } from '../../../entities/history-entry'
import type { ProjectRef } from '../../../entities/project'
import { readAggSettings } from './aggSettings'

export function useBuildAggregate({ host, project, branch, workflow }: ProjectRef) {
  const { start, ...build } = useBuild()
  // настройки читаем в момент клика: подписка на стор не нужна
  return { ...build, start: () => start(aggregateForm({ host, project, ref: branch ?? '', workflow: workflow ?? '', ...readAggSettings() })) }
}
