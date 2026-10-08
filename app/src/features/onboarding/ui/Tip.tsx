import { useEffect, useId, type ReactElement } from 'react'
import { Popover, PopoverAnchor, PopoverContent } from '../../../shared/ui/popover'
import { addReady, closeTip, removeReady, useTipOpen } from '../model/store'
import type { TipId } from '../model/tips'
import { TipBubble } from './TipBubble'

type Props = {
  id: TipId
  /** событие наступило; пока `false`, подсказка не регистрируется и не открывается — дерево то же, якорь не перемонтируется */
  when?: boolean
  /** один элемент, передающий ref и пропсы в DOM */
  children: ReactElement
}

/**
 * Подсказка у элемента. Пузырь порталится в `body`, но остаётся в React-дереве якоря, поэтому Radix-слои внутри модального Dialog считают его своим: клик и фокус работают.
 * Фокус не забирает, клик мимо не закрывает; Esc закрывает только подсказку (`stopPropagation` до `useReportKeys` и корневого обработчика).
 */
export function Tip({ id, when = true, children }: Props) {
  const titleId = useId()
  const open = useTipOpen(id) && when
  useEffect(() => {
    if (!when) return
    addReady(id)
    return () => removeReady(id)
  }, [id, when])
  return (
    <Popover open={open} onOpenChange={(next) => !next && closeTip()}>
      <PopoverAnchor asChild data-tip-active={open ? '' : undefined}>
        {children}
      </PopoverAnchor>
      <PopoverContent
        side="bottom"
        align="start"
        collisionPadding={8}
        aria-labelledby={titleId}
        className="w-[300px] border-mascot-border bg-mascot-bg p-3 data-[state=open]:animate-tip-in motion-reduce:animate-none"
        onOpenAutoFocus={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.stopPropagation()}
      >
        <TipBubble id={id} titleId={titleId} />
      </PopoverContent>
    </Popover>
  )
}
