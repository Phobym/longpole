import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// ok/run/mid/bad — метки статусов пайплайна и стабильности, скруглённые теги 4px;
// chip/chipCrit/chipBad — чипы шапки и панели отчёта: без рамки, на подложке, 6px (спека, § 3).
const badgeVariants = cva('inline-flex w-fit shrink-0 items-center justify-center gap-1 px-2 py-0.5 text-xs whitespace-nowrap', {
  variants: {
    variant: {
      secondary: 'rounded-md bg-secondary font-medium text-secondary-foreground',
      ok: 'rounded bg-pill-ok-bg font-medium text-pill-ok-fg',
      run: 'rounded bg-pill-run-bg font-medium text-pill-run-fg',
      mid: 'rounded bg-pill-mid-bg font-medium text-pill-mid-fg',
      bad: 'rounded bg-pill-bad-bg font-medium text-pill-bad-fg',
      chip: 'rounded-md bg-secondary text-muted-foreground [&_b]:font-semibold [&_b]:text-foreground',
      chipCrit: 'rounded-md bg-crit-bg text-crit-fg [&_b]:font-semibold [&_b]:text-crit-fg',
      chipBad: 'rounded-md bg-pill-bad-bg text-retry-fg [&_b]:font-semibold [&_b]:text-retry-fg',
    },
  },
  defaultVariants: { variant: 'secondary' },
})

export function Badge({ className, variant, ...props }: ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
