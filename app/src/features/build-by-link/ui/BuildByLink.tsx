import { useEffect, useId, useRef } from 'react'
import { fieldError, messageError } from '../../../shared/api'
import { errorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Input } from '../../../shared/ui/input'
import { linkForm, useBuild } from '../../../entities/history-entry'
import { useLink } from '../model/link'

export function BuildByLink() {
  const { t } = useTranslation()
  const id = useId()
  const input = useRef<HTMLInputElement>(null)
  const { url, setUrl, focusRequested, focused } = useLink()
  const { start, progress, error } = useBuild()
  const failure = fieldError(error, 'url') ?? messageError(error)

  useEffect(() => {
    if (!focusRequested) return
    input.current?.focus()
    focused()
  }, [focusRequested, focused])

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
        <Input ref={input} id={id} aria-invalid={failure !== undefined} aria-describedby={failure ? `${id}-error` : undefined} type="url" placeholder={t('form.link.placeholder')} value={url} onChange={(e) => setUrl(e.target.value)} />
        <Button type="submit" disabled={progress !== null} className="shrink-0">
          {progress ?? t('form.link.build')}
        </Button>
      </div>
      {failure && (
        <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
          {errorText(t, failure)}
        </p>
      )}
    </form>
  )
}
