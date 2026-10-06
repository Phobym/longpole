import { Dialog as DialogPrimitive } from 'radix-ui'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

export const Dialog = DialogPrimitive.Root
export const DialogTitle = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Title>) => (
  <DialogPrimitive.Title className={cn('text-lg font-semibold', className)} {...props} />
)
export const DialogDescription = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Description>) => (
  <DialogPrimitive.Description className={cn('text-sm text-muted-foreground', className)} {...props} />
)

/** Esc и клик по подложке закрывают (Radix зовёт `onOpenChange`); кнопки закрытия — у потребителя. */
export const DialogContent = ({ className, ...props }: ComponentProps<typeof DialogPrimitive.Content>) => (
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/40" />
    <DialogPrimitive.Content
      className={cn(
        'fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-full max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col gap-4 overflow-y-auto rounded-xl bg-chrome p-6 text-card-foreground shadow-[0_20px_60px_rgb(0_0_0/0.3),0_0_0_0.5px_rgb(0_0_0/0.2)] outline-hidden',
        className,
      )}
      {...props}
    />
  </DialogPrimitive.Portal>
)
