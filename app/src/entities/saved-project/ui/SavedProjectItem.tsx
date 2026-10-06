import { XIcon } from 'lucide-react'
import type { SavedProject } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { useTranslation } from '../../../shared/i18n'

type Props = { project: SavedProject; active: boolean; onOpen: () => void; onRemove: () => void }

// крестик — вне <button>: вложенные кнопки недопустимы
// активный пункт — как выделение в source list macOS: синяя подложка, белый текст
export function SavedProjectItem({ project, active, onOpen, onRemove }: Props) {
  const { t } = useTranslation()
  return (
    <li className="group flex items-center gap-1">
      <button
        type="button"
        aria-current={active ? 'page' : undefined}
        onClick={onOpen}
        className={cn(
          'flex min-w-0 flex-1 flex-col rounded-md px-2 py-1 text-left outline-hidden focus-visible:ring-[3px] focus-visible:ring-primary/35',
          active ? 'bg-primary text-primary-foreground' : 'hover:bg-accent',
        )}
      >
        <span translate="no" className="truncate text-sm">
          {project.name}
        </span>
        <span translate="no" className={cn('truncate text-[11px]', active ? 'text-primary-foreground/75' : 'text-muted-foreground')}>
          {project.host}
        </span>
      </button>
      <button
        type="button"
        aria-label={t('form.sidebar.removeProject', { name: project.name })}
        onClick={onRemove}
        className="rounded-md p-1 text-muted-foreground opacity-0 outline-hidden group-hover:opacity-100 hover:bg-accent focus-visible:opacity-100 focus-visible:ring-[3px] focus-visible:ring-primary/35"
      >
        <XIcon className="size-4" />
      </button>
    </li>
  )
}
