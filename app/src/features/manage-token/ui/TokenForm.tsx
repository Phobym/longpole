import { useId, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input } from '../../../shared/ui/input'
import { useSaveToken } from '../model/useSaveToken'

/** Хост и токен; `defaultHost` — выбранный хост без токена. Токен после сохранения прочитать нельзя нигде. */
export function TokenForm({ defaultHost }: { defaultHost: string }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const [host, setHost] = useState(defaultHost)
  const [token, setToken] = useState('')
  const { save, error } = useSaveToken()
  const failure = messageError(error)
  return (
    <form
      className="flex flex-col gap-2 rounded-md border bg-secondary/40 p-3"
      onSubmit={(e) => {
        e.preventDefault()
        void save(host, token)
      }}
    >
      <label htmlFor={`${id}-host`} className="text-sm font-medium">
        {t('form.host.label')}
      </label>
      <Input id={`${id}-host`} aria-invalid={failure !== undefined} aria-describedby={failure ? `${id}-error` : undefined} translate="no" placeholder={t('form.host.placeholder')} value={host} onChange={(e) => setHost(e.target.value)} />
      <label htmlFor={`${id}-token`} className="text-sm font-medium">
        {t('form.host.token')}
      </label>
      <Input id={`${id}-token`} type="password" autoComplete="new-password" autoCapitalize="off" autoCorrect="off" spellCheck={false} value={token} onChange={(e) => setToken(e.target.value)} />
      {failure && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
      <Button type="submit" className="self-start">
        {t('form.host.saveToken')}
      </Button>
    </form>
  )
}
