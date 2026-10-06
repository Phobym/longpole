import type { ReactNode } from 'react'

export const Section = ({ title, children }: { title: string; children: ReactNode }) => (
  <>
    <h3 className="mt-4 mb-1.5 text-[11px] font-semibold tracking-[.02em] text-muted-foreground">{title}</h3>
    {children}
  </>
)
