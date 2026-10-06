import type { Pipeline } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { durationShort, relativeTime } from '../../../shared/lib/format'
import { useLocale } from '../../../shared/i18n'
import { Badge } from '../../../shared/ui/badge'

const PILL = { success: 'ok', running: 'run', pending: 'run', failed: 'bad' } as const

type Props = {
  pipeline: Pipeline
  onOpen: () => void
  /** Идёт сборка этой строки: она неактивна и показывает `progress`. */
  busy: boolean
  progress: string
  /** Ошибка сборки этой строки. */
  error: string | null
}

// Статус и ошибка — вне <button>: содержимое кнопки читалка не озвучивает как live-регион.
export function PipelineRow({ pipeline, onOpen, busy, progress, error }: Props) {
  const locale = useLocale()
  const variant = PILL[pipeline.status as keyof typeof PILL] ?? 'mid'
  return (
    <li className="flex flex-col even:bg-muted">
      <button
        type="button"
        disabled={busy}
        onClick={onOpen}
        className="grid w-full grid-cols-[4rem_5.5rem_1fr_9rem_8.5rem_4.5rem] items-center gap-3 px-2.5 py-1 text-left text-[12.5px] focus-ring hover:bg-selected disabled:opacity-60"
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
        {error ?? (busy ? progress : null)}
      </p>
    </li>
  )
}
