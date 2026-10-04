import { fieldError, messageError } from '../../../shared/api'
import { errorText, useTranslation } from '../../../shared/i18n'
import { BuildAggregateButton, LastField, StatusFilter, useBuildAggregate } from '../../../features/build-aggregate'

type Props = { host: string; project: string; branch: string | undefined }

export function AggregateBlock(props: Props) {
  const { t } = useTranslation()
  const { start, progress, error } = useBuildAggregate(props)
  const failure = messageError(error)
  return (
    <section className="flex flex-col gap-3 border-t pt-4">
      <StatusFilter error={fieldError(error, 'statuses')} />
      <LastField error={fieldError(error, 'last')} />
      <BuildAggregateButton progress={progress} onClick={start} />
      {failure && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(t, failure)}
        </p>
      )}
    </section>
  )
}
