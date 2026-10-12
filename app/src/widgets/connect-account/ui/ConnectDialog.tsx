import { useState } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Dialog, DialogContent, DialogTitle } from '../../../shared/ui/dialog'
import { closeConnect, ConnectAccount, useConnectOpen } from '../../../features/manage-token'
import { useHostState } from '../../../entities/host'
import { PickProject } from './PickProject'

/** «Подключить аккаунт» из сайдбара и «Добавить проект»: вход, затем выбор проекта на новом хосте. */
export function ConnectDialog() {
  const open = useConnectOpen()
  return (
    <Dialog open={open} onOpenChange={(next) => !next && closeConnect()}>
      {/* тело внутри DialogContent: Radix размонтирует его при закрытии, и следующее открытие начинается с кнопок входа */}
      {/* описания нет: строка доверия уже внутри ConnectAccount */}
      <DialogContent className="max-w-md" aria-describedby={undefined}>
        <ConnectBody />
      </DialogContent>
    </Dialog>
  )
}

function ConnectBody() {
  const { t } = useTranslation()
  const [connected, setConnected] = useState(false)
  // после входа useOauthLogin/useSaveToken уже вызвали pickHost: tokenHost — только что подключённый хост
  const { tokenHost } = useHostState()
  return (
    <>
      <DialogTitle>{t('form.connect.title')}</DialogTitle>
      {connected && tokenHost ? <PickProject host={tokenHost} onAdded={closeConnect} /> : <ConnectAccount onConnected={() => setConnected(true)} />}
      <Button type="button" variant="ghost" className="self-end" onClick={closeConnect}>
        {t('form.cancel')}
      </Button>
    </>
  )
}
