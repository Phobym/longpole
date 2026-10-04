import { useEffect, useState } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { SEARCH_DEBOUNCE, useDebounced } from '../../../shared/lib/useDebounced'
import { Combobox } from '../../../shared/ui/combobox'
import { useBranches } from '../../../entities/project'

type Props = {
  host: string
  project: string
  /** Ветка, по которой сейчас построен список пайплайнов. */
  value: string | undefined
  /** Ветка изменилась (`''` — все ветки): выбор подсказки, Enter или уход фокуса. */
  onCommit: (ref: string) => void
}

export function BranchSelect({ host, project, value, onCommit }: Props) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(value ?? '')
  // пока текст не печатали, подсказки без фильтра: основная ветка первой
  const [search, setSearch] = useState('')
  useEffect(() => {
    setDraft(value ?? '')
    setSearch('')
  }, [value])
  // ошибки веток не показываем: та же причина видна в списке пайплайнов
  const { data } = useBranches(host, project, useDebounced(search, SEARCH_DEBOUNCE))
  return (
    <div className="flex flex-col gap-1.5">
      <span aria-hidden className="text-sm font-medium">
        {t('form.branch.label')}
      </span>
      <Combobox
        label={t('form.branch.label')}
        value={draft}
        options={data ?? []}
        onChange={(text) => {
          setDraft(text)
          setSearch(text)
        }}
        onCommit={(text) => {
          setDraft(text)
          if (text !== (value ?? '')) onCommit(text)
        }}
      />
    </div>
  )
}
