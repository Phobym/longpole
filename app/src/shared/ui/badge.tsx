import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// ok/run/mid/bad — пилюли статусов пайплайна (success, running, manual…, failed), как в старой форме.
const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap',
  {
    variants: {
      variant: {
        default: 'bg-primary text-primary-foreground',
        secondary: 'bg-secondary text-secondary-foreground',
        outline: 'border-border text-foreground',
        ok: 'bg-pill-ok-bg text-pill-ok-fg',
        run: 'bg-pill-run-bg text-pill-run-fg',
        mid: 'bg-pill-mid-bg text-pill-mid-fg',
        bad: 'bg-pill-bad-bg text-pill-bad-fg',
      },
    },
    defaultVariants: { variant: 'default' },
  },
)

export function Badge({ className, variant, ...props }: ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
