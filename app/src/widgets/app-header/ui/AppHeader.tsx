import { Link } from '@tanstack/react-router'
import { SettingsIcon } from 'lucide-react'
import { useTranslation } from '../../../shared/i18n'
import { buttonVariants } from '../../../shared/ui/button'
import { BuildByLink } from '../../../features/build-by-link'
import { useAddProjectOpen } from '../../../features/add-project'
import { TokenBlock } from '../../../features/manage-token'
import { Tip, useLeftReport } from '../../../features/onboarding'

/** Тулбар окна: поле ссылки, «Построить», шестерёнка. */
export function AppHeader() {
  const { t } = useTranslation()
  const leftReport = useLeftReport()
  const addProjectOpen = useAddProjectOpen()
  return (
    <header className="flex items-start gap-3 border-b bg-chrome px-4 py-2">
      <Tip id="header.link" when={leftReport}>
        <div className="min-w-0 flex-1">
          <BuildByLink
            tokenBlock={(host, retry) => (
              /* у подсказки один якорь: пока открыт диалог добавления, токен подсказывает он */
              <Tip id="token.create" when={!addProjectOpen}>
                <div>
                  <TokenBlock host={host} onSaved={retry} />
                </div>
              </Tip>
            )}
          />
        </div>
      </Tip>
      <Link to="/settings" aria-label={t('form.settingsButton')} className={buttonVariants({ variant: 'ghost', size: 'icon' })}>
        <SettingsIcon />
      </Link>
    </header>
  )
}
