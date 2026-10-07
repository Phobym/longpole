import { useId, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input, labelClasses } from '../../../shared/ui/input'
import { useHostProvider } from '../model/useHostProvider'
import { useSaveToken } from '../model/useSaveToken'

/** Токен для известного хоста прямо там, где он понадобился; после сохранения `onSaved` повторяет действие. */
export function TokenBlock({ host, onSaved }: { host: string; onSaved: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const [token, setToken] = useState('')
  const [saving, setSaving] = useState(false)
  const { save, error } = useSaveToken()
  const failure = messageError(error)
  const provider = useHostProvider(host)
  // страница создания токена с готовым именем и правами; `https` уходит в системный браузер через on_navigation
  const createUrl =
    provider === 'github'
      ? `https://${host}/settings/personal-access-tokens/new`
      : `https://${host}/-/user_settings/personal_access_tokens?name=pipeline-trace&scopes=read_api`
  return (
    <form
      className="flex flex-col gap-2 rounded-md border border-warn bg-secondary/40 p-3"
      onSubmit={async (e) => {
        e.preventDefault()
        setSaving(true)
        const saved = await save(host, token).finally(() => setSaving(false))
        if (saved) onSaved()
      }}
    >
      <p className="text-sm font-medium">{t('form.token.need', { host })}</p>
      <p className="text-xs text-muted-foreground">
        {t(provider === 'github' ? 'form.token.scopeGithub' : 'form.token.scope')}{' '}
        <a href={createUrl} target="_blank" rel="noopener noreferrer" className="text-link hover:underline">
          {t('form.token.create', { host })}
        </a>
      </p>
      <label htmlFor={id} className={labelClasses}>
        {t('form.host.token')}
      </label>
      <Input id={id} type="password" autoComplete="new-password" autoCapitalize="off" autoCorrect="off" spellCheck={false} aria-invalid={failure !== undefined} aria-describedby={failure ? `${id}-error` : undefined} value={token} onChange={(e) => setToken(e.target.value)} />
      <p className="text-xs text-muted-foreground">{t('form.token.stored')}</p>
      {failure && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
      <Button type="submit" className="self-start" disabled={saving}>
        {t('form.host.saveToken')}
      </Button>
    </form>
  )
}
