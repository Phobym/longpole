import { useErrorText, useTranslation } from '../../../shared/i18n'
import { type ErrorBody } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { labelClasses } from '../../../shared/ui/input'
import { STATUSES, toggleAny, toggleStatus, useAggStatuses } from '../model/aggSettings'

export function StatusFilter({ error }: { error?: ErrorBody }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const statuses = useAggStatuses()
  const any = statuses === null
  return (
    <fieldset className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
      <legend className={cn('mb-1.5', labelClasses)}>{t('form.aggregate.statuses')}</legend>
      {STATUSES.map((status) => (
        <label key={status} className="flex items-center gap-1.5 text-sm">
          <input
            type="checkbox"
            className="size-3.5 accent-primary"
            checked={statuses?.includes(status) ?? false}
            disabled={any}
            onChange={() => toggleStatus(status)}
          />
          {status}
        </label>
      ))}
      <label className="flex items-center gap-1.5 text-sm">
        <input type="checkbox" className="size-3.5 accent-primary" checked={any} onChange={toggleAny} />
        {t('form.aggregate.any')}
      </label>
      {error && (
        <p role="alert" className="basis-full text-sm text-destructive">
          {errorText(error)}
        </p>
      )}
    </fieldset>
  )
}
