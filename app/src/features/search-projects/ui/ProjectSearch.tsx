import { useId } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Input } from '../../../shared/ui/input'
import { setSearchText, useSearchText } from '../model/search'

export function ProjectSearch() {
  const { t } = useTranslation()
  const id = useId()
  const text = useSearchText()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.projects.search')}
      </label>
      <Input id={id} type="search" placeholder={t('form.projects.searchPlaceholder')} value={text} onChange={(e) => setSearchText(e.target.value)} />
    </div>
  )
}
