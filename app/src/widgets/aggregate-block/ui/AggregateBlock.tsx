import { fieldError, messageError } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
import { BuildAggregateButton, LastField, StatusFilter, useBuildAggregate } from '../../../features/build-aggregate'
import type { ProjectRef } from '../../../entities/project'

export function AggregateBlock(target: ProjectRef) {
  const errorText = useErrorText()
  const { start, busy, progressLabel, error } = useBuildAggregate(target)
  const failure = messageError(error)
  const workflowError = fieldError(error, 'workflow')
  return (
    <section className="flex flex-col gap-3 border-t pt-4">
      <StatusFilter error={fieldError(error, 'statuses')} />
      <LastField error={fieldError(error, 'last')} />
      {workflowError && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(workflowError)}
        </p>
      )}
      <BuildAggregateButton busy={busy} progressLabel={progressLabel} onClick={start} />
      {failure && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
    </section>
  )
}
