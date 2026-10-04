import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// ok/run/mid/bad — пилюли статусов пайплайна (success, running, manual…, failed), как в старой форме;
// chip/chipCrit/chipBad — чипы шапки и панели отчёта (обычный, оранжевый, красный).
const badgeVariants = cva(
  'inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-full border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap',
  {
    variants: {
      variant: {
        secondary: 'bg-secondary text-secondary-foreground',
        ok: 'bg-pill-ok-bg text-pill-ok-fg',
        run: 'bg-pill-run-bg text-pill-run-fg',
        mid: 'bg-pill-mid-bg text-pill-mid-fg',
        bad: 'bg-pill-bad-bg text-pill-bad-fg',
        chip: 'border-border bg-card font-normal text-muted-foreground [&_b]:font-semibold [&_b]:text-foreground',
        chipCrit: 'border-crit-soft bg-card font-normal text-crit-fg [&_b]:font-semibold [&_b]:text-crit-fg',
        chipBad: 'border-pill-bad-bg bg-card font-normal text-retry-fg [&_b]:font-semibold [&_b]:text-retry-fg',
      },
    },
    defaultVariants: { variant: 'secondary' },
  },
)

export function Badge({ className, variant, ...props }: ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
