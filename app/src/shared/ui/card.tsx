import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

export const Card = ({ className, ...props }: ComponentProps<'div'>) => (
  <div className={cn('flex flex-col gap-4 rounded-xl border bg-card p-6 text-card-foreground shadow-sm', className)} {...props} />
)

export const CardHeader = ({ className, ...props }: ComponentProps<'div'>) => (
  <div className={cn('flex flex-col gap-1.5', className)} {...props} />
)

export const CardTitle = ({ className, ...props }: ComponentProps<'div'>) => (
  <div className={cn('leading-none font-semibold', className)} {...props} />
)

export const CardContent = ({ className, ...props }: ComponentProps<'div'>) => <div className={className} {...props} />
