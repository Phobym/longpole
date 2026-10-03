import { cn } from '../../../shared/lib/cn'

/** Ссылка в GitLab в новой вкладке; без адреса — прочерк. */
export const GitlabLink = ({ url, className, children }: { url: string | null; className?: string; children: React.ReactNode }) =>
  url ? (
    <a href={url} target="_blank" rel="noopener" className={cn('text-primary', className)}>
      {children}
    </a>
  ) : (
    '—'
  )
