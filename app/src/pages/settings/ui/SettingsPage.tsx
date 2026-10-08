import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { apiError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { useGoBack } from '../../../shared/lib/useGoBack'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { labelClasses } from '../../../shared/ui/input'
import { ThemeSwitch } from '../../../features/choose-theme'
import { TipsSettings } from '../../../features/onboarding'
import { RemoveTokenButton, TokenForm } from '../../../features/manage-token'
import { hostsQuery } from '../../../entities/host'
import { LanguageSwitch } from '../../../widgets/language-switch'

export function SettingsPage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { data: hosts = [], error } = useQuery(hostsQuery)
  const [adding, setAdding] = useState(false)
  const failure = apiError(error)
  const back = useGoBack()
  return (
    <Card>
      <div className="flex items-center gap-3">
        <Button type="button" variant="ghost" size="sm" onClick={back}>
          {t('form.settings.back')}
        </Button>
        <h1 className="text-[17px] font-semibold">{t('form.settings.title')}</h1>
      </div>

      <section className="flex flex-col gap-2">
        <h2 className={labelClasses}>{t('form.settings.hosts')}</h2>
        {failure && (
          <p role="alert" className="text-sm text-destructive">
            {errorText(failure)}
          </p>
        )}
        {hosts.length === 0 && !failure && <p className="text-sm text-muted-foreground">{t('form.settings.noHosts')}</p>}
        <ul className="flex flex-col overflow-hidden rounded-lg border empty:hidden">
          {hosts.map((h) => (
            <li key={h.host} className="flex items-center gap-3 px-2.5 py-1.5 text-sm even:bg-muted">
              <span translate="no" className="flex-1 truncate">
                {h.host}
              </span>
              {h.provider && (
                <span translate="no" className="text-[11px] text-muted-foreground">
                  {h.provider === 'github' ? 'GitHub' : 'GitLab'}
                </span>
              )}
              <span className="text-[11px] text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              {h.source === 'keychain' && <RemoveTokenButton host={h.host} />}
            </li>
          ))}
        </ul>
        {adding ? (
          <>
            <TokenForm defaultHost="" onSaved={() => setAdding(false)} />
            <Button type="button" variant="ghost" className="self-start" onClick={() => setAdding(false)}>
              {t('form.cancel')}
            </Button>
          </>
        ) : (
          <Button type="button" variant="outline" className="self-start" onClick={() => setAdding(true)}>
            {t('form.settings.addHost')}
          </Button>
        )}
        {navigator.platform.startsWith('Linux') && <p className="text-xs text-muted-foreground">{t('form.settings.linuxHint')}</p>}
      </section>

      <section className="flex flex-col gap-2">
        <h2 className={labelClasses}>{t('form.settings.theme')}</h2>
        <ThemeSwitch />
      </section>

      <section className="flex flex-col gap-2">
        <h2 className={labelClasses}>{t('form.settings.tips')}</h2>
        <TipsSettings />
      </section>

      <section className="flex flex-col gap-2">
        <h2 className={labelClasses}>{t('form.language')}</h2>
        <LanguageSwitch />
      </section>
    </Card>
  )
}
