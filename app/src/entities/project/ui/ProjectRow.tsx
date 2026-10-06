import { type Project } from '../../../shared/api'
import { relativeTime } from '../../../shared/lib/format'
import { useLocale, useTranslation } from '../../../shared/i18n'
import { Badge } from '../../../shared/ui/badge'

export function ProjectRow({ project, onOpen }: { project: Project; onOpen: () => void }) {
  const { t } = useTranslation()
  const locale = useLocale()
  return (
    <li className="even:bg-muted">
      <button
        type="button"
        onClick={onOpen}
        className="flex w-full flex-col gap-0.5 px-2.5 py-1.5 text-left focus-ring hover:bg-selected"
      >
        <span translate="no" className="font-medium">
          {project.name}
        </span>
        <span translate="no" className="text-xs text-muted-foreground">
          {project.fullPath}
        </span>
        <span className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
          {project.defaultBranch && (
            <Badge translate="no" variant="chip">
              {project.defaultBranch}
            </Badge>
          )}
          {project.lastActivityAt && (
            <span>{t('form.projects.activity', { time: relativeTime(project.lastActivityAt, locale) })}</span>
          )}
        </span>
      </button>
    </li>
  )
}
