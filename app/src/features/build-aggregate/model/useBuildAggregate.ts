import { aggregateForm, useBuild } from '../../../entities/history-entry'
import { useAggSettings } from './aggSettings'

type Target = { host: string; project: string; branch: string | undefined }

export function useBuildAggregate({ host, project, branch }: Target) {
  const { last, statuses } = useAggSettings()
  const { start, progress, error } = useBuild()
  return { progress, error, start: () => start(aggregateForm({ host, project, ref: branch ?? '', last, statuses })) }
}
