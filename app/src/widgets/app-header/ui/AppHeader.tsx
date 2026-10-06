import { Link } from '@tanstack/react-router'
import { SettingsIcon } from 'lucide-react'
import { useTranslation } from '../../../shared/i18n'
import { buttonVariants } from '../../../shared/ui/button'
import { BuildByLink } from '../../../features/build-by-link'
import { TokenBlock } from '../../../features/manage-token'

/** Тулбар окна: поле ссылки, «Построить», шестерёнка. */
export function AppHeader() {
  const { t } = useTranslation()
  return (
    <header className="flex items-start gap-3 border-b bg-chrome px-4 py-2">
      <div className="min-w-0 flex-1">
        <BuildByLink tokenBlock={(host, retry) => <TokenBlock host={host} onSaved={retry} />} />
      </div>
      <Link to="/settings" aria-label={t('form.settingsButton')} className={buttonVariants({ variant: 'ghost', size: 'icon' })}>
        <SettingsIcon />
      </Link>
    </header>
  )
}
