import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'

export type DepDir = 'up' | 'down'

/** Метка у связанной с выбранной строки: «↑ нужна для X» или «↓ ждёт X». */
export function DepTag({ dir, name }: { dir: DepDir; name: string }) {
  const { t } = useTranslation()
  return (
    <span
      className={cn(
        'absolute top-[calc(50%-8px)] right-1.5 z-[1] bg-card pl-1.5 text-[11px] leading-4 font-semibold whitespace-nowrap',
        dir === 'up' ? 'text-dep-up' : 'text-dep-down',
      )}
    >
      {t(dir === 'up' ? 'report.needFor' : 'report.waits')} <span translate="no">{name}</span>
    </span>
  )
}
