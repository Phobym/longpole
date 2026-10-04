import { useNavigate } from '@tanstack/react-router'
import { type HistoryEntry } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { fillAggSettings } from '../../../features/build-aggregate'
import { fillLink } from '../../../features/build-by-link'
import { ClearHistoryButton, RemoveEntryButton } from '../../../features/history-actions'
import { resetSearch } from '../../../features/search-projects'
import { pickHost } from '../../../entities/host'
import { HistoryEntryItem, useHistory } from '../../../entities/history-entry'
import { useOpenProject } from '../../../entities/project'

export function HistorySidebar() {
  const { t } = useTranslation()
  const { data: entries = [] } = useHistory()
  const navigate = useNavigate()
  const openProject = useOpenProject()

  // Запись заполняет сторы через публичные API слайсов и переходит на нужный экран.
  const open = ({ host, form }: HistoryEntry) => {
    if (form.mode === 'link') {
      fillLink(form.url)
      void navigate({ to: '/' })
      return
    }
    pickHost(host)
    resetSearch()
    fillAggSettings(form)
    void openProject({ host, fullPath: form.project, ref: form.ref || undefined })
  }

  return (
    <aside className="flex w-72 shrink-0 flex-col gap-2 overflow-y-auto border-r bg-card p-4">
      <div className="flex items-center justify-between">
        <h2 className="text-sm font-semibold">{t('form.history.title')}</h2>
        {entries.length > 0 && <ClearHistoryButton />}
      </div>
      {entries.length === 0 && <p className="text-sm text-muted-foreground">{t('form.history.empty')}</p>}
      <ul className="flex flex-col gap-0.5">
        {entries.map((entry) => (
          <li key={entry.at} className="flex items-center gap-1">
            <HistoryEntryItem entry={entry} onOpen={() => open(entry)} />
            <RemoveEntryButton entry={entry} />
          </li>
        ))}
      </ul>
    </aside>
  )
}
