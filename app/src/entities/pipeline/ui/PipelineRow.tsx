import type { Pipeline } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { durationShort, relativeTime } from '../../../shared/lib/format'
import { useLocale } from '../../../shared/i18n'
import { Badge } from '../../../shared/ui/badge'

const PILL = { success: 'ok', running: 'run', pending: 'run', failed: 'bad' } as const

type Props = {
  pipeline: Pipeline
  onOpen: () => void
  /** Прогресс сборки этой строки; пока он есть, строка неактивна. */
  progress: string | null
  /** Ошибка сборки этой строки. */
  error: string | null
}

// Статус и ошибка — вне <button>: содержимое кнопки читалка не озвучивает как live-регион.
export function PipelineRow({ pipeline, onOpen, progress, error }: Props) {
  const locale = useLocale()
  const variant = PILL[pipeline.status as keyof typeof PILL] ?? 'mid'
  return (
    <li className="flex flex-col">
      <button
        type="button"
        disabled={progress !== null}
        onClick={onOpen}
        className="grid w-full grid-cols-[4rem_5.5rem_1fr_9rem_7rem_4.5rem] items-center gap-3 rounded-md border bg-card px-3 py-2 text-left text-sm outline-hidden hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-60"
      >
        <span className="font-medium">#{pipeline.iid}</span>
        <Badge variant={variant}>{pipeline.status}</Badge>
        <span translate="no" className="truncate">
          {pipeline.commit?.title ?? '—'}
        </span>
        <span translate="no" className="truncate text-muted-foreground">
          {pipeline.author ?? '—'}
        </span>
        <span className="text-muted-foreground">{relativeTime(pipeline.createdAt, locale)}</span>
        <span className="text-right text-muted-foreground">{durationShort(pipeline.duration)}</span>
      </button>
      <p aria-live="polite" className={cn('px-3 text-xs empty:hidden', error ? 'text-destructive' : 'text-muted-foreground')}>
        {error ?? progress}
      </p>
    </li>
  )
}
