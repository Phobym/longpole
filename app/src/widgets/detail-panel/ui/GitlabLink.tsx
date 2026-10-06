import type { ReactNode } from 'react'
import { cn } from '../../../shared/lib/cn'

/** Ссылка в GitLab в новой вкладке; без адреса — прочерк. */
export const GitlabLink = ({ url, className, children }: { url: string | null; className?: string; children: ReactNode }) =>
  url ? (
    <a href={url} target="_blank" rel="noopener" className={cn('text-link', className)}>
      {children}
    </a>
  ) : (
    '—'
  )
