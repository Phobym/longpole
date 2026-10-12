import { useTranslation } from '../i18n'
import { Button } from './button'

/** «Повторить» после `buildTimeout` (спека поставки, § 14). */
export function RetryButton({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation()
  return (
    <Button type="button" size="sm" className="self-start" onClick={onClick}>
      {t('form.retry')}
    </Button>
  )
}
