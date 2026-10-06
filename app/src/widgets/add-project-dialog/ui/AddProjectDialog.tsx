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
  const { t } = useTranslation()
  const errorText = useErrorText()
  const id = useId()
  const open = useAddProjectOpen()
  const [input, setInput] = useState('')
  const [browse, setBrowse] = useState(false)
  const { add, busy, error, reset } = useAddProject()
  const { tokenHost } = useHostState()
  const failure = messageError(error)
  const needsToken = failure?.code === 'noToken' ? failure.params.host : null

  const close = () => {
    closeAddProject()
    setInput('')
    setBrowse(false)
    reset()
  }
  const pick = async () => {
    const dir = await pickFolder()
    if (!dir) return
    setInput(dir)
    add(dir)
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent>
        <DialogTitle>{t('form.addProject.title')}</DialogTitle>
        <DialogDescription>{t('form.addProject.hint')}</DialogDescription>
        <form
          className="flex flex-col gap-2"
          onSubmit={(e) => {
            e.preventDefault()
            add(input)
          }}
        >
          <div className="flex gap-2">
            <Input id={id} autoFocus translate="no" placeholder={t('form.addProject.placeholder')} aria-invalid={failure !== undefined && !needsToken} aria-describedby={failure ? `${id}-error` : undefined} value={input} onChange={(e) => setInput(e.target.value)} />
            <Button type="button" variant="outline" className="shrink-0" onClick={() => void pick()}>
              {t('form.addProject.pickFolder')}
            </Button>
          </div>
          {failure && !needsToken && (
            <p id={`${id}-error`} role="alert" className="text-sm text-destructive">
              {errorText(failure)}
            </p>
          )}
          {needsToken && <TokenBlock host={needsToken} onSaved={() => add(input)} />}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={close}>
              {t('form.cancel')}
            </Button>
            <Button type="submit" disabled={busy || input.trim() === ''}>
              {busy ? t('form.loading') : t('form.addProject.add')}
            </Button>
          </div>
        </form>
        <button type="button" aria-expanded={browse} className="self-start text-sm text-primary hover:underline" onClick={() => setBrowse((b) => !b)}>
          {t('form.addProject.findOnHost')} {browse ? '▾' : '▸'}
        </button>
        {browse && (
          <div className="flex flex-col gap-3 border-t pt-3">
            <HostSelect onChange={resetSearch} />
            {tokenHost !== null && <ProjectsPanel host={tokenHost} onPick={(project) => add(`https://${tokenHost}/${project.fullPath}`)} />}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
