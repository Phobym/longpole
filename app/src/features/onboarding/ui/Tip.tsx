import { useEffect, type ReactElement } from 'react'
import { Popover, PopoverAnchor, PopoverContent } from '../../../shared/ui/popover'
import { addReady, closeTip, removeReady, useTipOpen } from '../model/store'
import type { TipId } from '../model/tips'
import { TipBubble } from './TipBubble'

type Props = {
  id: TipId
  /** событие наступило; `false` — ребёнок без обёртки */
  when?: boolean
  /** один элемент, передающий ref и пропсы в DOM */
  children: ReactElement
}

/**
 * Подсказка у элемента. Поповер — в дереве якоря, а не в корне: внутри модального Dialog поповер из корня не получает кликов и фокуса.
 * Фокус не забирает, клик мимо не закрывает; Esc закрывает только подсказку (`stopPropagation` до `useReportKeys` и корневого обработчика).
 */
export function Tip({ id, when = true, children }: Props) {
  const open = useTipOpen(id)
  useEffect(() => {
    if (!when) return
    addReady(id)
    return () => removeReady(id)
  }, [id, when])
  if (!when) return children
  return (
    <Popover open={open} onOpenChange={(next) => !next && closeTip()}>
      <PopoverAnchor asChild data-tip-active={open ? '' : undefined}>
        {children}
      </PopoverAnchor>
      <PopoverContent
        side="bottom"
        align="start"
        collisionPadding={8}
        className="w-[300px] border-mascot-border bg-mascot-bg p-3 data-[state=open]:animate-tip-in motion-reduce:animate-none"
        onOpenAutoFocus={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.stopPropagation()}
      >
        <TipBubble id={id} />
      </PopoverContent>
    </Popover>
  )
}
