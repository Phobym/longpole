import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

export const Input = ({ className, type, ...props }: ComponentProps<'input'>) => (
  <input
    type={type}
    className={cn(
      'h-9 w-full min-w-0 rounded-md border border-input bg-card px-3 py-1 text-sm outline-none transition-colors placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive',
      className,
    )}
    {...props}
  />
)
