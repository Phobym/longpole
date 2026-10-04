import { Popover as PopoverPrimitive } from 'radix-ui'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

export const Popover = PopoverPrimitive.Root
export const PopoverTrigger = PopoverPrimitive.Trigger
export const PopoverAnchor = PopoverPrimitive.Anchor

export const PopoverContent = ({ className, align = 'center', sideOffset = 4, ...props }: ComponentProps<typeof PopoverPrimitive.Content>) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      align={align}
      sideOffset={sideOffset}
      className={cn('z-50 w-72 rounded-md border bg-popover p-4 text-popover-foreground shadow-md outline-hidden', className)}
      {...props}
    />
  </PopoverPrimitive.Portal>
)
