import { useQuery } from '@tanstack/react-query'
import { useNavigate, useRouter } from '@tanstack/react-router'
import { useState } from 'react'
import { apiError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Card } from '../../../shared/ui/card'
import { ThemeSwitch } from '../../../features/choose-theme'
import { RemoveTokenButton, TokenForm } from '../../../features/manage-token'
import { hostsQuery } from '../../../entities/host'
import { LanguageSwitch } from '../../../widgets/language-switch'

export function SettingsPage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const navigate = useNavigate()
  const router = useRouter()
  const { data: hosts = [], error } = useQuery(hostsQuery)
  const [adding, setAdding] = useState(false)
  const failure = apiError(error)
  const back = () => {
    if (router.history.canGoBack()) router.history.back()
    else void navigate({ to: '/' })
  }
  return (
    <Card>
      <div className="flex items-center gap-3">
        <Button type="button" variant="ghost" size="sm" onClick={back}>
          {t('form.settings.back')}
        </Button>
        <h1 className="text-xl font-semibold">{t('form.settings.title')}</h1>
      </div>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.settings.hosts')}</h2>
        {failure && (
          <p role="alert" className="text-sm text-destructive">
            {errorText(failure)}
          </p>
        )}
        {hosts.length === 0 && !failure && <p className="text-sm text-muted-foreground">{t('form.settings.noHosts')}</p>}
        <ul className="flex flex-col gap-1.5">
          {hosts.map((h) => (
            <li key={h.host} className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm">
              <span translate="no" className="flex-1 truncate font-medium">
                {h.host}
              </span>
              <span className="text-xs text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              {h.source === 'keychain' && <RemoveTokenButton host={h.host} />}
            </li>
          ))}
        </ul>
        {adding ? (
          <TokenForm defaultHost="" onSaved={() => setAdding(false)} />
        ) : (
          <Button type="button" variant="outline" className="self-start" onClick={() => setAdding(true)}>
            {t('form.settings.addHost')}
          </Button>
        )}
        {navigator.platform.startsWith('Linux') && <p className="text-xs text-muted-foreground">{t('errors.keychainUnavailable')}</p>}
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.settings.theme')}</h2>
        <ThemeSwitch />
      </section>

      <section className="flex flex-col gap-2">
        <h2 className="text-sm font-semibold">{t('form.language')}</h2>
        <LanguageSwitch />
      </section>
    </Card>
  )
}
