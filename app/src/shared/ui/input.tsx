import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// поле macOS: 26px, внутренняя полупиксельная тень, синий ореол на фокусе (спека, § 3)
export const inputClasses =
  'h-[26px] w-full min-w-0 rounded-md border border-black/18 bg-card px-2 text-[13px] shadow-[inset_0_0.5px_1px_rgb(0_0_0/0.08)] focus-ring transition-colors placeholder:text-muted-foreground focus-visible:border-primary disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive dark:border-white/12 dark:bg-[#1a1a1a]'

/** Подпись поля формы: 11px капсом, приглушённая (спека, § 4). */
export const labelClasses = 'text-[11px] font-medium tracking-[.02em] text-muted-foreground uppercase'

export const Input = ({ className, ...props }: ComponentProps<'input'>) => (
  <input className={cn(inputClasses, className)} {...props} />
)
