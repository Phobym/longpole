import { apiErrorText, useTranslation } from '../../../shared/i18n'
import { Card } from '../../../shared/ui/card'
import { BuildByLink } from '../../../features/build-by-link'
import { HostSelect, RemoveTokenButton, TokenForm } from '../../../features/manage-token'
import { resetSearch } from '../../../features/search-projects'
import { useHostState } from '../../../entities/host'
import { ProjectsPanel } from '../../../widgets/projects-panel'

export function ProjectsPage() {
  const { t } = useTranslation()
  const { host, hasToken, ready, error } = useHostState()
  return (
    <Card>
      <h1 className="text-xl font-semibold">{t('form.projects.title')}</h1>
      <BuildByLink />
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {apiErrorText(t, error)}
        </p>
      )}
      {ready && (
        <>
          <HostSelect onChange={resetSearch}>{host !== null && hasToken && <RemoveTokenButton host={host} />}</HostSelect>
          {host !== null && hasToken ? (
            <ProjectsPanel host={host} />
          ) : (
            <>
              <p className="text-sm text-muted-foreground">{t('form.host.hint')}</p>
              <TokenForm key={host ?? ''} defaultHost={host ?? ''} />
            </>
          )}
        </>
      )}
    </Card>
  )
}
