import { Link } from '@tanstack/react-router'
import { SettingsIcon } from 'lucide-react'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { buttonVariants } from '../../../shared/ui/button'
import { BuildByLink } from '../../../features/build-by-link'

export function AppHeader() {
  const { t } = useTranslation()
  return (
    <header className="flex items-start gap-3 border-b bg-card px-6 py-3">
      <div className="min-w-0 flex-1">
        <BuildByLink />
      </div>
      <Link to="/settings" aria-label={t('form.settingsButton')} className={cn(buttonVariants({ variant: 'ghost', size: 'icon' }), 'mt-6')}>
        <SettingsIcon />
      </Link>
    </header>
  )
}
