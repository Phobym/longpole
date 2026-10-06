import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// inset-grouped карточка macOS: 10px, полупрозрачная hairline, едва заметная тень (спека, § 3)
export const Card = ({ className, ...props }: ComponentProps<'div'>) => (
  <div
    className={cn('flex flex-col gap-4 rounded-[10px] border border-black/10 bg-card px-[18px] py-4 text-card-foreground shadow-[0_1px_2px_rgb(0_0_0/0.04)] dark:border-white/8', className)}
    {...props}
  />
)

export const CardHeader = ({ className, ...props }: ComponentProps<'div'>) => (
  <div className={cn('flex flex-col gap-1.5', className)} {...props} />
)

export const CardTitle = ({ className, ...props }: ComponentProps<'div'>) => (
  <div className={cn('leading-none font-semibold', className)} {...props} />
)

export const CardContent = ({ className, ...props }: ComponentProps<'div'>) => <div className={className} {...props} />
