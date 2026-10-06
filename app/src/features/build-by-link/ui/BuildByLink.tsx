import { useEffect, useId, useRef, type ReactNode } from 'react'
import { fieldError, messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input } from '../../../shared/ui/input'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { markFocusHandled, setLinkUrl, useFocusRequested, useLinkUrl } from '../model/link'

export function BuildByLink({ tokenBlock }: { tokenBlock?: (host: string, retry: () => void) => ReactNode }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const input = useRef<HTMLInputElement>(null)
  const url = useLinkUrl()
  const focusRequested = useFocusRequested()
  const { start, busy, progressLabel, error } = useBuild()
  const failure = fieldError(error, 'url') ?? messageError(error)

  useEffect(() => {
    if (!focusRequested) return
    input.current?.focus()
    markFocusHandled()
  }, [focusRequested])

  return (
    <form
      noValidate // ошибку ссылки показывает ядро, а не подсказка браузера
      className="flex flex-col gap-1.5"
      onSubmit={(e) => {
        e.preventDefault()
        start(linkForm(url))
      }}
    >
      <label htmlFor={id} className="text-sm font-medium">
        {t('form.link.label')}
      </label>
      <div className="flex gap-2">
        <Input
          ref={input}
          id={id}
          aria-invalid={failure !== undefined}
          aria-describedby={failure ? `${id}-error` : undefined}
          type="url"
          placeholder={t('form.link.placeholder')}
          value={url}
          onChange={(e) => setLinkUrl(e.target.value)}
        />
        <Button type="submit" disabled={busy} className="shrink-0">
          {busy ? progressLabel : t('form.link.build')}
        </Button>
      </div>
      {failure && !(failure.code === 'noToken' && tokenBlock) && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(failure)}
        </p>
      )}
      {failure?.code === 'noToken' && tokenBlock?.(failure.params.host, () => start(linkForm(url)))}
    </form>
  )
}
