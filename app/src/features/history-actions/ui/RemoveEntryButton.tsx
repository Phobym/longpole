import { XIcon } from 'lucide-react'
import { type HistoryEntry } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { entryLabel } from '../../../entities/history-entry'
import { useHistoryActions } from '../model/useHistoryActions'

/** Отдельная кнопка рядом с записью, а не внутри неё: клик по ней не должен ещё и открывать запись. */
export function RemoveEntryButton({ entry }: { entry: HistoryEntry }) {
  const { t } = useTranslation()
  const { remove } = useHistoryActions()
  const label = entryLabel(t, entry)
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon"
      className="size-7 shrink-0 text-muted-foreground"
      aria-label={t('form.history.remove', { label })}
      onClick={() => void remove(entry.at)}
    >
      <XIcon />
    </Button>
  )
}
