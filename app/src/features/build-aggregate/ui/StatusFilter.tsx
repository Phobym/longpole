import { errorText, useTranslation } from '../../../shared/i18n'
import { type ErrorBody } from '../../../shared/api'
import { ANY, STATUSES, useAggSettings } from '../model/aggSettings'

export function StatusFilter({ error }: { error?: ErrorBody }) {
  const { t } = useTranslation()
  const { statuses, toggleStatus, toggleAny } = useAggSettings()
  const any = statuses.includes(ANY)
  return (
    <fieldset className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
      <legend className="mb-1.5 text-sm font-medium">{t('form.aggregate.statuses')}</legend>
      {STATUSES.map((status) => (
        <label key={status} className="flex items-center gap-1.5 text-sm">
          <input
            type="checkbox"
            className="accent-primary"
            checked={!any && statuses.includes(status)}
            disabled={any}
            onChange={() => toggleStatus(status)}
          />
          {status}
        </label>
      ))}
      <label className="flex items-center gap-1.5 text-sm">
        <input type="checkbox" className="accent-primary" checked={any} onChange={toggleAny} />
        {t('form.aggregate.any')}
      </label>
      {error && (
        <p role="alert" className="basis-full text-sm text-destructive">
          {errorText(t, error)}
        </p>
      )}
    </fieldset>
  )
}
