import { XIcon } from 'lucide-react'
import type { SavedProject } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { useTranslation } from '../../../shared/i18n'

type Props = { project: SavedProject; active: boolean; onOpen: () => void; onRemove: () => void }

// крестик — вне <button>: вложенные кнопки недопустимы
export function SavedProjectItem({ project, active, onOpen, onRemove }: Props) {
  const { t } = useTranslation()
  return (
    <li className="group flex items-center gap-1">
      <button
        type="button"
        aria-current={active ? 'page' : undefined}
        onClick={onOpen}
        className={cn(
          'flex min-w-0 flex-1 flex-col rounded-md px-2 py-1.5 text-left outline-hidden hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring',
          active && 'bg-selected',
        )}
      >
        <span translate="no" className="truncate text-sm font-medium">
          {project.name}
        </span>
        <span translate="no" className="truncate text-xs text-muted-foreground">
          {project.host}
        </span>
      </button>
      <button
        type="button"
        aria-label={t('form.sidebar.removeProject', { name: project.name })}
        onClick={onRemove}
        className="rounded-md p-1 text-muted-foreground opacity-0 outline-hidden group-hover:opacity-100 hover:bg-accent focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-ring"
      >
        <XIcon className="size-4" />
      </button>
    </li>
  )
}
