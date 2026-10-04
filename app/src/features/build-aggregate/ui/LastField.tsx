import { useId } from 'react'
import { errorText, useTranslation } from '../../../shared/i18n'
import { type ErrorBody } from '../../../shared/api'
import { Input } from '../../../shared/ui/input'
import { useAggSettings } from '../model/aggSettings'

export function LastField({ error }: { error?: ErrorBody }) {
  const { t } = useTranslation()
  const id = useId()
  const { last, setLast } = useAggSettings()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.aggregate.last')}
      </label>
      <Input id={id} aria-invalid={error !== undefined} aria-describedby={error ? `${id}-error` : undefined} type="number" min={1} max={500} className="w-32" value={last} onChange={(e) => setLast(e.target.value)} />
      {error && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(t, error)}
        </p>
      )}
    </div>
  )
}
