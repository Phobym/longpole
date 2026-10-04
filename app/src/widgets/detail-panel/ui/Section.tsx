import type { ReactNode } from 'react'

export const Section = ({ title, children }: { title: string; children: ReactNode }) => (
  <>
    <h3 className="mt-4 mb-1.5 text-[10.5px] font-semibold tracking-[.06em] text-muted-foreground uppercase">{title}</h3>
    {children}
  </>
)
