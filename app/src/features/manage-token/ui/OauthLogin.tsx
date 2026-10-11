import { useId, useState } from 'react'
import { messageError, type Provider } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input, labelClasses } from '../../../shared/ui/input'
import { Giraffe } from '../../../shared/ui/giraffe'
import { CLOUD_HOSTS, registerAppUrl } from '../model/oauth'
import { useOauthLogin } from '../model/useOauthLogin'
import { OauthWaiting } from './OauthWaiting'

const linkClasses = 'text-link hover:underline'

/** Вход через OAuth рядом с формой PAT. `provider` известен — показываем ссылку на регистрацию приложения. */
export function OauthLogin({ host, provider, onSaved }: { host: string; provider?: Provider; onSaved?: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const [clientId, setClientId] = useState('')
  const { login, cancel, waiting, code, error } = useOauthLogin(host)
  const failure = messageError(error)
  const cloud = CLOUD_HOSTS.includes(host)

  if (waiting) return <OauthWaiting host={host} code={code} onCancel={cancel} />

  return (
    <div className="flex flex-col gap-2">
      {!cloud && host && (
        <>
          <label htmlFor={`${id}-client`} className={labelClasses}>
            {t('form.oauth.clientId')}
          </label>
          <Input id={`${id}-client`} translate="no" autoCapitalize="off" autoCorrect="off" spellCheck={false} value={clientId} onChange={(e) => setClientId(e.target.value)} />
          <p className="text-xs text-muted-foreground">
            {t('form.oauth.clientIdHint', { host })}{' '}
            {provider && (
              <a href={registerAppUrl(host, provider)} target="_blank" rel="noopener noreferrer" className={linkClasses}>
                {t('form.oauth.register')}
              </a>
            )}
          </p>
        </>
      )}
      <Button
        type="button"
        className="self-start"
        disabled={!host}
        onClick={async () => {
          if (await login(cloud ? null : clientId.trim() || null)) onSaved?.()
        }}
      >
        {t('form.oauth.login', { host: host || '…' })}
      </Button>
      {failure && (
        <p role="alert" className="flex items-start gap-2 text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
    </div>
  )
}
