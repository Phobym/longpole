import { fieldError, messageError } from '../../../shared/api'
import { useErrorText } from '../../../shared/i18n'
import { BuildAggregateButton, LastField, StatusFilter, useBuildAggregate } from '../../../features/build-aggregate'
import type { ProjectRef } from '../../../entities/project'
import { Giraffe } from '../../../shared/ui/giraffe'
import { Tip } from '../../../features/onboarding'

export function AggregateBlock(target: ProjectRef) {
  const errorText = useErrorText()
  const { start, busy, progress, progressLabel, error } = useBuildAggregate(target)
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
      <div className="flex items-end gap-3">
        <Tip id="project.screen">
          <div className="inline-flex self-start">
            <BuildAggregateButton busy={busy} progressLabel={progressLabel} onClick={start} />
          </div>
        </Tip>
        {/* слот под жирафа занят всегда: строка не прыгает, когда он появляется */}
        <div className="h-[46px] w-[40px] shrink-0" aria-hidden>
          {busy && <Giraffe pose="loading" size={40} progress={progress ?? undefined} />}
        </div>
      </div>
      {failure && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
    </section>
  )
}
