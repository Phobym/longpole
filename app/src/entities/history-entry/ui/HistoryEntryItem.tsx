import type { TFunction } from 'i18next'
import { type HistoryEntry } from '../../../shared/api'
import { dateTime } from '../../../shared/lib/format'
import { useLocale, useTranslation } from '../../../shared/i18n'

/** Подпись записи: проект и метка на текущем языке (`null` — «все пайплайны», как `meta.label`). */
export const entryLabel = (t: TFunction, { label }: HistoryEntry) => `${label.project} · ${label.label ?? t('report.allPipelines')}`

export function HistoryEntryItem({ entry, onOpen }: { entry: HistoryEntry; onOpen: () => void }) {
  const { t } = useTranslation()
  const locale = useLocale()
  const label = entryLabel(t, entry)
  return (
    <button
      type="button"
      onClick={onOpen}
      className="flex min-w-0 flex-1 flex-col gap-0.5 rounded-md px-2 py-1.5 text-left outline-hidden hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring"
    >
      <span translate="no" className="truncate text-sm font-medium">
        {label}
      </span>
      <span className="flex gap-2 text-xs text-muted-foreground">
        <span translate="no" className="truncate">
          {entry.host}
        </span>
        <span className="shrink-0">{dateTime(entry.at, locale)}</span>
      </span>
    </button>
  )
}
