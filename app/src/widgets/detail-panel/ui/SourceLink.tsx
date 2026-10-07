import type { ReactNode } from 'react'
import type { Provider } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'

/** Имя источника в подписи ссылки: бренд, не переводится. */
export const sourceName = (provider: Provider) => (provider === 'github' ? 'GitHub' : 'GitLab')

/** Ссылка в GitLab или GitHub в новой вкладке; без адреса — прочерк. */
export const SourceLink = ({ url, className, children }: { url: string | null; className?: string; children: ReactNode }) =>
  url ? (
    <a href={url} target="_blank" rel="noopener" className={cn('text-link', className)}>
      {children}
    </a>
  ) : (
    '—'
  )
