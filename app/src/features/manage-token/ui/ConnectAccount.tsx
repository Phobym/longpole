import { useEffect, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Giraffe } from '../../../shared/ui/giraffe'
import { ProviderIcon } from '../../../shared/ui/provider-icon'
import { useOauthLogin } from '../model/useOauthLogin'
import { OauthWaiting } from './OauthWaiting'
import { TokenForm } from './TokenForm'

/**
 * Вход с первого экрана и из «Подключить аккаунт» (спека, § 5): github.com и gitlab.com в один клик,
 * свой сервер и токен вручную — через `TokenForm`, где есть и OAuth с Client ID, и PAT.
 */
export function ConnectAccount({ onConnected, onWaitingChange }: { onConnected?: () => void; onWaitingChange?: (waiting: boolean) => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const [server, setServer] = useState(false)
  const [last, setLast] = useState<'github' | 'gitlab' | null>(null)
  const github = useOauthLogin('github.com')
  const gitlab = useOauthLogin('gitlab.com')
  // родитель прячет своё оформление, пока `OauthWaiting` занимает место
  const waiting = github.waiting || gitlab.waiting
  useEffect(() => onWaitingChange?.(waiting), [waiting, onWaitingChange])
  // ошибка — только у последнего начатого входа: у каждого useOauthLogin своя
  const failure = messageError(last === 'github' ? github.error : last === 'gitlab' ? gitlab.error : null)

  if (server) {
    return (
      <div className="flex w-full flex-col gap-2 text-left">
        <Button type="button" variant="ghost" size="sm" className="self-start" onClick={() => { setServer(false); setLast(null) }}>
          {t('form.connect.back')}
        </Button>
        <TokenForm defaultHost="" onSaved={onConnected} />
      </div>
    )
  }
  if (github.waiting) return <OauthWaiting host="github.com" code={github.code} onCancel={github.cancel} className="items-center text-center" />
  if (gitlab.waiting) return <OauthWaiting host="gitlab.com" code={gitlab.code} onCancel={gitlab.cancel} className="items-center text-center" />

  const start = async (provider: 'github' | 'gitlab') => {
    setLast(provider)
    const login = provider === 'github' ? github.login : gitlab.login
    if (await login(null)) onConnected?.()
  }
  return (
    <div className="flex w-full flex-col gap-2">
      {/* белый на тёмном — 15:1; на оранжевом GitLab было бы ≈2,9:1, поэтому GitLab — outline с оранжевым логотипом */}
      <Button type="button" className="h-8 from-[#24292f] to-[#1d1d1f] text-white dark:from-[#f5f5f7] dark:to-[#e5e5ea] dark:text-[#1d1d1f]" onClick={() => void start('github')}>
        <ProviderIcon provider="github" />
        {t('form.connect.github')}
      </Button>
      <Button type="button" variant="outline" className="h-8" onClick={() => void start('gitlab')}>
        <ProviderIcon provider="gitlab" className="text-[#e24329]" />
        {t('form.connect.gitlab')}
      </Button>
      <Button type="button" variant="outline" className="h-8" onClick={() => setServer(true)}>
        {t('form.connect.server')}
      </Button>
      <p className="text-xs text-muted-foreground"><span aria-hidden>🔒</span> {t('form.connect.trust')}</p>
      <button type="button" className="text-xs text-link hover:underline" onClick={() => setServer(true)}>
        {t('form.connect.manual')}
      </button>
      {failure && (
        <p role="alert" className="flex items-start gap-2 text-left text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
    </div>
  )
}
