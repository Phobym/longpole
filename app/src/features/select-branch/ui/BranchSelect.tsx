import { useState } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { SEARCH_DEBOUNCE, useDebounced } from '../../../shared/lib/useDebounced'
import { Combobox } from '../../../shared/ui/combobox'
import { labelClasses } from '../../../shared/ui/input'
import { useBranches, type ProjectRef } from '../../../entities/project'

type Props = ProjectRef & {
  /** Ветка изменилась (`''` — все ветки): выбор подсказки, Enter или уход фокуса. */
  onCommit: (branch: string) => void
}

export function BranchSelect({ host, project, branch, onCommit }: Props) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(branch ?? '')
  // пока текст не печатали, подсказки без фильтра: основная ветка первой
  const [search, setSearch] = useState('')
  // Ветка снаружи сменилась (запись истории, другой проект): набранное устарело. Сброс прямо в рендере, без эффекта.
  const [seen, setSeen] = useState(branch)
  if (seen !== branch) {
    setSeen(branch)
    setDraft(branch ?? '')
    setSearch('')
  }
  // ошибки веток не показываем: та же причина видна в списке пайплайнов
  const { data } = useBranches(host, project, useDebounced(search, SEARCH_DEBOUNCE))
  return (
    <div className="flex flex-col gap-1.5">
      <span aria-hidden className={labelClasses}>
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
          if (text !== (branch ?? '')) onCommit(text)
        }}
      />
    </div>
  )
}
