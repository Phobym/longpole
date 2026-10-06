import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// push-кнопки macOS: лёгкий градиент и полупиксельная тень, высота 26px (спека, § 3)
export const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-[13px] font-medium whitespace-nowrap transition-colors outline-hidden focus-visible:ring-[3px] focus-visible:ring-primary/35 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default:
          'bg-linear-to-b from-[#3a97ff] to-primary text-primary-foreground shadow-[0_0.5px_1px_rgb(0_0_0/0.25),inset_0_0.5px_0_rgb(255_255_255/0.25)] hover:brightness-105',
        destructive: 'bg-destructive text-background hover:bg-destructive/90',
        outline:
          'border border-black/12 bg-linear-to-b from-card to-secondary shadow-[0_0.5px_1px_rgb(0_0_0/0.15)] hover:from-secondary dark:border-white/10 dark:from-[#5a5a5e] dark:to-[#4a4a4e] dark:hover:from-[#4a4a4e]',
        secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
        ghost: 'hover:bg-accent hover:text-accent-foreground',
      },
      size: {
        default: 'h-[26px] px-3',
        sm: 'h-[22px] gap-1.5 px-2.5 text-xs',
        icon: 'size-[26px]',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  },
)

export const Button = ({ className, variant, size, ...props }: ComponentProps<'button'> & VariantProps<typeof buttonVariants>) => (
  <button className={cn(buttonVariants({ variant, size }), className)} {...props} />
)
