import { useEffect, useId, type ReactElement } from 'react'
import { Popover, PopoverAnchor, PopoverContent } from '../../../shared/ui/popover'
import { addReady, closeTip, removeReady, useTipOpen } from '../model/store'
import type { TipId } from '../model/tips'
import { TipBubble } from './TipBubble'

type Props = {
  /** `null` — здесь подсказки нет: не регистрируется и не открывается, дерево то же (для строк, где id считается на лету) */
  id: TipId | null
  /** событие наступило; пока `false`, подсказка не регистрируется и не открывается — дерево то же, якорь не перемонтируется */
  when?: boolean
  /** с какой стороны якоря пузырь */
  side?: 'top' | 'right' | 'bottom' | 'left'
  /** обводка внутри якоря — для строк в контейнере с overflow-hidden, где внешнюю обрезало бы; по умолчанию снаружи, чтобы не ложилась на текст */
  inset?: boolean
  /** один элемент, передающий ref и пропсы в DOM */
  children: ReactElement
}

/**
 * Подсказка у элемента. Пузырь порталится в `body`, но остаётся в React-дереве якоря, поэтому Radix-слои внутри модального Dialog считают его своим: клик и фокус работают.
 * Фокус не забирает, клик мимо не закрывает; Esc закрывает только подсказку (`stopPropagation` до `useReportKeys` и корневого обработчика).
 */
export function Tip({ id, when = true, side = 'bottom', inset = false, children }: Props) {
  const titleId = useId()
  const open = useTipOpen(id) && when
  useEffect(() => {
    if (!when || id === null) return
    addReady(id)
    return () => removeReady(id)
  }, [id, when])
  return (
    <Popover open={open} onOpenChange={(next) => !next && closeTip()}>
      <PopoverAnchor asChild data-tip-active={open ? (inset ? 'inset' : '') : undefined}>
        {children}
      </PopoverAnchor>
      <PopoverContent
        side={side}
        align="start"
        // зазор 10px от внешнего края обводки: снаружи она выступает на 6px (2px + offset 4px)
        sideOffset={inset ? 10 : 16}
        collisionPadding={16}
        aria-labelledby={titleId}
        className="w-[300px] rounded-lg border-mascot-border bg-mascot-bg p-3 data-[state=open]:animate-tip-in motion-reduce:animate-none"
        onOpenAutoFocus={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.stopPropagation()}
      >
        {id !== null && <TipBubble id={id} titleId={titleId} />}
      </PopoverContent>
    </Popover>
  )
}
