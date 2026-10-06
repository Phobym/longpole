import { useId, useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../../shared/ui/dialog'
import { Input } from '../../../shared/ui/input'
import { closeAddProject, pickFolder, useAddProject, useAddProjectOpen } from '../../../features/add-project'
import { HostSelect, TokenBlock } from '../../../features/manage-token'
import { ProjectsPanel, resetSearch } from '../../../features/search-projects'
import { useHostState } from '../../../entities/host'

/** Одно поле на ссылку или путь, «Выбрать папку…», разворачиваемый поиск по хосту; токен запрашивается на месте. */
export function AddProjectDialog() {
  const open = useAddProjectOpen()
  return (
    <Dialog open={open} onOpenChange={(next) => !next && closeAddProject()}>
      {/* тело внутри DialogContent: Radix размонтирует его при закрытии, и следующее открытие начинается с чистого поля */}
      <DialogContent>
        <AddProjectBody />
      </DialogContent>
    </Dialog>
  )
}

function AddProjectBody() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const [input, setInput] = useState('')
  const [browse, setBrowse] = useState(false)
  const { add, busy, error, lastInput } = useAddProject()
  const { tokenHost } = useHostState()
  const failure = messageError(error)
  const needsToken = failure?.code === 'noToken' ? failure.params.host : null
  const showError = failure !== undefined && !needsToken

  const submit = (value: string) => {
    setInput(value)
    add(value)
  }
  const pick = async () => {
    const dir = await pickFolder()
    if (dir) submit(dir)
  }

  return (
    <>
      <DialogTitle>{t('form.addProject.title')}</DialogTitle>
      <DialogDescription>{t('form.addProject.hint')}</DialogDescription>
      <form
        id={`${id}-form`}
        className="flex flex-col gap-2"
        onSubmit={(e) => {
          e.preventDefault()
          add(input)
        }}
      >
        <div className="flex gap-2">
          <Input id={id} autoFocus translate="no" placeholder={t('form.addProject.placeholder')} aria-invalid={showError} aria-describedby={showError ? `${id}-error` : undefined} value={input} onChange={(e) => setInput(e.target.value)} />
          <Button type="button" variant="outline" className="shrink-0" disabled={busy} onClick={() => void pick()}>
            {t('form.addProject.pickFolder')}
          </Button>
        </div>
        {showError && (
          <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
            {errorText(failure)}
          </p>
        )}
      </form>
      {/* своя форма токена — соседом, не внутри: вложенный submit всплыл бы и повторно отправил внешнюю */}
      {needsToken && <TokenBlock host={needsToken} onSaved={() => add(lastInput ?? input)} />}
      {/* кнопки вне формы и привязаны к ней через form=, чтобы блок токена стоял сразу под полем */}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" disabled={busy} onClick={closeAddProject}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={`${id}-form`} disabled={busy || input.trim() === ''}>
          {busy ? t('form.loading') : t('form.addProject.add')}
        </Button>
      </div>
      <button type="button" aria-expanded={browse} aria-controls={`${id}-browse`} className="self-start text-sm text-link hover:underline" onClick={() => setBrowse((b) => !b)}>
        {t('form.addProject.findOnHost')} <span aria-hidden>{browse ? '▾' : '▸'}</span>
      </button>
      {browse && (
        <div id={`${id}-browse`} className="flex flex-col gap-3 border-t pt-3">
          <HostSelect onChange={resetSearch} />
          {tokenHost !== null && <ProjectsPanel host={tokenHost} onPick={(project) => !busy && submit(`https://${tokenHost}/${project.fullPath}`)} />}
        </div>
      )}
    </>
  )
}
