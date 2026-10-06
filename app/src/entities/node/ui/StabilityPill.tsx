import type { Stability } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { Badge } from '../../../shared/ui/badge'

const PILL = 'px-[7px] py-0 text-[10.5px] leading-4 group-data-[selected]:bg-white/20 group-data-[selected]:text-primary-foreground'

/** «стабильна» / «↻ k из N»: жёлтая при ретраях менее чем в 30% запусков, красная иначе. */
export function StabilityPill({ stability }: { stability: Stability | undefined }) {
  const { t } = useTranslation()
  if (!stability?.present) return null
  const { retried, present } = stability
  if (retried === 0) return <Badge variant="ok" className={PILL}>{t('report.stable')}</Badge>
  return (
    <Badge variant={retried / present < 0.3 ? 'mid' : 'bad'} className={PILL} title={t('report.retriedTitle', { retried, present, count: present })}>
      {t('report.retried', { retried, present })}
    </Badge>
  )
}
