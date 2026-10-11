import type { UserCode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { Button, buttonVariants } from '../../../shared/ui/button'
import { Giraffe } from '../../../shared/ui/giraffe'
import { GITHUB_APP_INSTALL } from '../model/oauth'

/** Ожидание входа: у GitHub — код и «Скопировать код и открыть», у GitLab браузер открывает ядро. */
export function OauthWaiting({ host, code, onCancel, className }: { host: string; code: UserCode | null; onCancel: () => void; className?: string }) {
  const { t } = useTranslation()
  return (
    <div role="status" className={cn('flex flex-col items-start gap-2', className)}>
      <Giraffe pose="loading" size={48} />
      {code ? (
        <>
          <p className="text-sm">{t('form.oauth.enterCode')}</p>
          <p translate="no" className="font-mono text-2xl tracking-widest select-all">
            {code.userCode}
          </p>
          {/* ссылка, а не кнопка: копируем по клику, а переход уходит в системный браузер через on_navigation */}
          {/* webview может запретить буфер обмена — код остаётся на экране для ручного ввода */}
          <a href={code.verificationUri} target="_blank" rel="noopener noreferrer" className={buttonVariants()} onClick={() => navigator.clipboard.writeText(code.userCode).catch(() => {})}>
            {t('form.connect.copyOpen', { host })}
          </a>
          {host === 'github.com' && (
            <a href={GITHUB_APP_INSTALL} target="_blank" rel="noopener noreferrer" className="text-xs text-link hover:underline">
              {t('form.oauth.installOrg')}
            </a>
          )}
        </>
      ) : (
        <p className="text-sm">{t('form.oauth.finishInBrowser')}</p>
      )}
      <Button type="button" variant="ghost" onClick={onCancel}>
        {t('form.cancel')}
      </Button>
    </div>
  )
}
