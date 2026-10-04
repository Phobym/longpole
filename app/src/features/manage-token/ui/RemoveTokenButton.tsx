import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { useRemoveToken } from '../model/useRemoveToken'

export function RemoveTokenButton({ host }: { host: string }) {
  const { t } = useTranslation()
  const remove = useRemoveToken()
  return (
    <Button type="button" variant="outline" className="shrink-0" onClick={() => void remove(host)}>
      {t('form.host.removeToken')}
    </Button>
  )
}
