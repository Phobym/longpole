import { useQuery } from '@tanstack/react-query'
import { login, type HostInfo, type Provider } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { ProviderIcon } from '../../../shared/ui/provider-icon'
import { hostsQuery } from '../../../entities/host'
import { useLogout } from '../model/useLogout'

const providerName = (provider: Provider | null) => (provider === 'github' ? 'GitHub' : 'GitLab')

/**
 * Приветствие Selfhosted (спека, § 10): по кнопке на каждый хост инстанса, вход переходом всей страницы.
 * Возврат на `/`: после входа хост со входом один, и `/` открывает выбор проекта на нём.
 */
export function WelcomeLogin() {
  const { t } = useTranslation()
  const { data: hosts = [] } = useQuery(hostsQuery)
  return (
    <div className="flex w-full flex-col gap-2">
      {hosts.map((h) => (
        <Button key={h.host} type="button" onClick={() => login(h.host, '/')}>
          <ProviderIcon provider={h.provider} />
          <span>{t('form.welcome.loginWith', { provider: providerName(h.provider), host: h.host })}</span>
        </Button>
      ))}
    </div>
  )
}

/** Вход в хост инстанса в «Настройки → Хосты»: «Вошли как @login · Выйти» или «Войти». */
export function HostAccount({ host }: { host: HostInfo }) {
  const { t } = useTranslation()
  const logout = useLogout()
  if (host.login === null) {
    return (
      <Button type="button" variant="outline" className="shrink-0" onClick={() => login(host.host)}>
        {t('form.settings.signIn')}
      </Button>
    )
  }
  return (
    <span className="flex shrink-0 items-center gap-1 text-[11px] text-muted-foreground">
      <span translate="no">{t('form.settings.signedInAs', { login: host.login })}</span>·
      <button type="button" className="text-link hover:underline" onClick={() => void logout(host.host)}>
        {t('form.settings.signOut')}
      </button>
    </span>
  )
}
