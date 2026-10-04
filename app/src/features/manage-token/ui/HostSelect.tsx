import { useId, type ReactNode } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../../shared/ui/select'
import { pickHost, useHostState } from '../../../entities/host'

// Radix Select не принимает пустое значение у пункта, поэтому «+ Добавить хост…» — служебная строка.
const ADD_HOST = '__add__'

export function HostSelect({ onChange, children }: { onChange: () => void; children?: ReactNode }) {
  const { t } = useTranslation()
  const id = useId()
  const { host, saved, hasToken } = useHostState()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.host.label')}
      </label>
      <div className="flex gap-2">
        <Select
          value={host ?? ADD_HOST}
          onValueChange={(value) => {
            pickHost(value === ADD_HOST ? null : value)
            onChange()
          }}
        >
          <SelectTrigger id={id} translate="no">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {saved.map((h) => (
              <SelectItem key={h} value={h}>
                {t('form.host.saved', { host: h })}
              </SelectItem>
            ))}
            {/* хост из истории или ссылки, токена для которого ещё нет: временный пункт, чтобы список показывал текущий хост */}
            {host !== null && !hasToken && <SelectItem value={host}>{host}</SelectItem>}
            <SelectItem value={ADD_HOST}>{t('form.host.add')}</SelectItem>
          </SelectContent>
        </Select>
        {children}
      </div>
    </div>
  )
}
