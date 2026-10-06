import { Navigate } from '@tanstack/react-router'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { openAddProject } from '../../../features/add-project'
import { useSavedProjects } from '../../../entities/saved-project'
import { useSettings } from '../../../entities/settings'

/** Нет проектов — пустое состояние; есть — последний открытый (или первый). */
export function HomePage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { projects, ready, error } = useSavedProjects()
  const { data: settings } = useSettings()
  if (error) {
    return (
      <p role="alert" className="text-sm text-destructive">
        {errorText(error)}
      </p>
    )
  }
  if (!ready || !settings) return <p className="text-sm text-muted-foreground">{t('form.loading')}</p>
  if (projects.length === 0) {
    return (
      <Card className="items-center py-16 text-center">
        <h1 className="text-xl font-semibold">{t('form.empty.title')}</h1>
        <p className="max-w-md text-sm text-muted-foreground">{t('form.empty.text')}</p>
        <Button type="button" onClick={openAddProject}>
          {t('form.empty.add')}
        </Button>
      </Card>
    )
  }
  const last = settings.lastProject
  const target = projects.find((p) => p.host === last?.host && p.path === last?.path) ?? projects[0]
  return (
    <Navigate to="/project/$host/$" params={{ host: target.host, _splat: target.path }} search={{ ref: undefined }} state={{ name: target.name }} replace />
  )
}
