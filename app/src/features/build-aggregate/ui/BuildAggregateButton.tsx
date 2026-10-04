import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { useAggLast } from '../model/aggSettings'

type Props = { busy: boolean; progressLabel: string; onClick: () => void }

/** «Агрегат по N пайплайнам»: N — то, что сейчас в поле, и склоняется по нему; во время сборки — прогресс. */
export function BuildAggregateButton({ busy, progressLabel, onClick }: Props) {
  const { t } = useTranslation()
  const last = useAggLast()
  return (
    <Button type="button" disabled={busy} className="self-start" onClick={onClick}>
      {busy ? progressLabel : t('form.aggregate.build', { count: Number(last) || 0, value: last || 0 })}
    </Button>
  )
}
