import { useState } from 'react'
import { Navigate } from '@tanstack/react-router'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { Giraffe } from '../../../shared/ui/giraffe'
import { openAddProject } from '../../../features/add-project'
import { ConnectAccount } from '../../../features/manage-token'
import { PickProject } from '../../../widgets/connect-account'
import { useHostState } from '../../../entities/host'
import { useSavedProjects } from '../../../entities/saved-project'
import { useSettings } from '../../../entities/settings'
import { homeStep } from '../model/step'

/** Без проектов и хостов — вход, хост есть — выбор проекта, проекты есть — последний открытый (или первый). */
export function HomePage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { projects, ready, error } = useSavedProjects()
  const { data: settings } = useSettings()
  // ошибка чтения хостов — как будто их нет: сбой связки ключей не должен закрывать вход (спека, § 2)
  const { saved: hosts, tokenHost, ready: hostsReady } = useHostState()
  const [waiting, setWaiting] = useState(false)
  if (error) {
    return (
      <p role="alert" className="text-sm text-destructive">
        {errorText(error)}
      </p>
    )
  }
  // хосты нужны только чтобы выбрать welcome/pick; при наличии проектов редирект их не ждёт
  if (!ready || !settings || (projects.length === 0 && !hostsReady)) {
    return (
      <p role="status" className="text-sm text-muted-foreground">
        {t('form.loading')}
      </p>
    )
  }
  const step = homeStep(projects.length, hosts.length)
  if (step === 'welcome') {
    return (
      <Card className="m-auto w-full max-w-[340px] items-center gap-3 py-8 text-center">
        {!waiting && <Giraffe size={96} />}
        <h1 className="text-xl font-semibold">{t('form.welcome.title')}</h1>
        {!waiting && <p className="text-sm text-muted-foreground">{t('form.welcome.text')}</p>}
        <ConnectAccount onWaitingChange={setWaiting} />
        {!waiting && (
          <Button type="button" variant="ghost" size="sm" className="text-muted-foreground" onClick={openAddProject}>
            {t('form.welcome.skip')}
          </Button>
        )}
      </Card>
    )
  }
  if (step === 'pick') {
    return (
      <Card className="m-auto w-full max-w-[480px]">
        <PickProject host={tokenHost ?? hosts[0]?.host ?? ''} />
      </Card>
    )
  }
  const last = settings.lastProject
  const target = projects.find((p) => p.host === last?.host && p.path === last?.path) ?? projects[0]
  return (
    <Navigate to="/project/$host/$" params={{ host: target.host, _splat: target.path }} search={{ ref: undefined }} state={{ name: target.name }} replace />
  )
}
