import { useId } from 'react'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { type ErrorBody } from '../../../shared/api'
import { Input } from '../../../shared/ui/input'
import { setAggLast, useAggLast } from '../model/aggSettings'

export function LastField({ error }: { error?: ErrorBody }) {
  const { t } = useTranslation()
  const id = useId()
  const errorText = useErrorText()
  const last = useAggLast()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.aggregate.last')}
      </label>
      <Input id={id} aria-invalid={error !== undefined} aria-describedby={error ? `${id}-error` : undefined} type="number" min={1} max={500} className="w-32" value={last} onChange={(e) => setAggLast(e.target.value)} />
      {error && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(error)}
        </p>
      )}
    </div>
  )
}
