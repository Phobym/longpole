import { Link, useNavigate, useParams } from '@tanstack/react-router'
import { PlusIcon } from 'lucide-react'
import type { ReactNode } from 'react'
import { findReport, type HistoryEntry } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { Button } from '../../../shared/ui/button'
import { ProviderIcon } from '../../../shared/ui/provider-icon'
import { openAddProject } from '../../../features/add-project'
import { fillAggSettings } from '../../../features/build-aggregate'
import { fillLink } from '../../../features/build-by-link'
import { ClearHistoryButton, RemoveEntryButton } from '../../../features/history-actions'
import { openConnect } from '../../../features/manage-token'
import { resetSearch } from '../../../features/search-projects'
import { pickHost, useHostState } from '../../../entities/host'
import { HistoryEntryItem, useHistory } from '../../../entities/history-entry'
import { useOpenProject } from '../../../entities/project'
import { SavedProjectItem, useRemoveProject, useSavedProjects } from '../../../entities/saved-project'
import { Giraffe } from '../../../shared/ui/giraffe'

function Section({ title, action, className, children }: { title: string; action?: ReactNode; className?: string; children: ReactNode }) {
  return (
    <section className={cn('flex flex-col gap-2', className)}>
      <div className="flex items-center justify-between">
        <h2 className="px-2 text-[11px] font-semibold text-muted-foreground">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  )
}

export function ProjectsSidebar() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const navigate = useNavigate()
  const openProject = useOpenProject()
  const params = useParams({ strict: false })
  const { projects, error: projectsError } = useSavedProjects()
  const removeProject = useRemoveProject()
  const { entries, error: historyError } = useHistory()
  const { host: currentHost, saved: hosts } = useHostState()

  // Готовый отчёт ещё в памяти — сразу на него; иначе запись заполняет сторы и ведёт на нужный экран, как раньше.
  const openEntry = async ({ host, form, request }: HistoryEntry) => {
    const cached = await findReport(host, request)
    if (cached.ok && cached.value !== null) {
      void navigate({ to: '/report/$id', params: { id: String(cached.value) } })
      return
    }
    if (form.mode === 'link') {
      // поле ссылки — в шапке, она всегда на экране; `/` увёл бы на последний проект
      fillLink(form.url)
      return
    }
    if (host !== currentHost) resetSearch()
    pickHost(host)
    fillAggSettings(form)
    void openProject({ host, project: form.project, branch: form.ref || undefined, workflow: form.workflow || undefined })
  }

  const error = projectsError ?? historyError
  return (
    <aside className="flex w-60 shrink-0 flex-col gap-5 overflow-y-auto border-r bg-sidebar px-2.5 py-3">
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {errorText(error)}
        </p>
      )}
      <Section
        title={t('form.sidebar.projects')}
        action={
          <Button type="button" variant="ghost" size="sm" onClick={openAddProject}>
            {t('form.sidebar.add')}
          </Button>
        }
      >
        <ul className="flex flex-col gap-0.5">
          {projects.map((project) => (
            <SavedProjectItem
              key={`${project.host}/${project.path}`}
              project={project}
              active={params.host === project.host && params._splat === project.path}
              onOpen={() => void openProject({ host: project.host, project: project.path, branch: undefined, name: project.name })}
              onRemove={() => void removeProject({ host: project.host, path: project.path })}
            />
          ))}
        </ul>
      </Section>
      <Section title={t('form.sidebar.recent')} action={entries.length > 0 && <ClearHistoryButton />}>
        {entries.length === 0 && !historyError && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Giraffe size={32} />
            {t('form.history.empty')}
          </p>
        )}
        <ul className="flex flex-col gap-0.5">
          {entries.map((entry) => (
            <li key={entry.at} className="flex items-center gap-1">
              <HistoryEntryItem entry={entry} onOpen={() => void openEntry(entry)} />
              <RemoveEntryButton entry={entry} />
            </li>
          ))}
        </ul>
      </Section>
      {/* прижат к низу: аккаунты всегда на виду, второй подключается отсюда (спека, § 6) */}
      <Section title={t('form.sidebar.accounts')} className="mt-auto">
        <ul className="flex flex-col gap-0.5">
          {hosts.map((h) => (
            <li key={h.host}>
              <Link to="/settings" className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-accent">
                <ProviderIcon provider={h.provider} className="text-muted-foreground" />
                <span translate="no" className="min-w-0 flex-1 truncate">
                  {h.host}
                </span>
                <span className="text-[11px] text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              </Link>
            </li>
          ))}
          <li>
            <button type="button" className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-link hover:bg-accent" onClick={openConnect}>
              <PlusIcon aria-hidden className="size-4" />
              {t('form.sidebar.connect')}
            </button>
          </li>
        </ul>
      </Section>
    </aside>
  )
}
