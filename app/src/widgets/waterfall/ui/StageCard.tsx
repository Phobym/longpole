import type { ReactNode } from 'react'
import { cn } from '../../../shared/lib/cn'
import { Card } from '../../../shared/ui/card'

/** Карточка стейджа; `cont` — продолжение после строк downstream-пайплайна, без заголовка. */
export function StageCard({ cont, children }: { cont: boolean; children: ReactNode }) {
  return (
    <Card className={cn('mt-px mb-2.5 gap-0 overflow-hidden rounded-[10px] border-0 p-0 shadow-none ring-1 ring-border', cont && '-mt-[9px] rounded-t-none')}>
      {children}
    </Card>
  )
}
