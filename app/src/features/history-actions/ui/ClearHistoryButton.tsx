import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { useHistoryActions } from '../model/useHistoryActions'

export function ClearHistoryButton() {
  const { t } = useTranslation()
  const { clear } = useHistoryActions()
  return (
    <Button type="button" variant="ghost" size="sm" onClick={() => void clear()}>
      {t('form.history.clear')}
    </Button>
  )
}
