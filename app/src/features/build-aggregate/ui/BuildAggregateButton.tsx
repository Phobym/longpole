import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { useAggSettings } from '../model/aggSettings'

/** «Агрегат по N пайплайнам»: N — то, что сейчас в поле, и склоняется по нему; во время сборки — прогресс. */
export function BuildAggregateButton({ progress, onClick }: { progress: string | null; onClick: () => void }) {
  const { t } = useTranslation()
  const { last } = useAggSettings()
  return (
    <Button type="button" disabled={progress !== null} className="self-start" onClick={onClick}>
      {progress ?? t('form.aggregate.build', { count: Number(last) || 0, value: last || 0 })}
    </Button>
  )
}
