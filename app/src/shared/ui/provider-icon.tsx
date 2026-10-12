import { ServerIcon } from 'lucide-react'
import type { Provider } from '../api'
import { cn } from '../lib/cn'

// брендовых иконок в lucide нет; контуры упрощены до 16×16
const PATHS: Record<Provider, string> = {
  github:
    'M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.49c-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.5 7.5 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 8 0Z',
  gitlab:
    'm15.73 6.33-.02-.06L13.53.6a.57.57 0 0 0-1.08.06l-1.47 4.5H5.02L3.55.66A.57.57 0 0 0 2.47.6L.29 6.27l-.02.06a4.05 4.05 0 0 0 1.34 4.68l3.4 2.55 1.69 1.27 1.02.77a.69.69 0 0 0 .83 0l1.02-.77 1.69-1.27 3.42-2.56a4.05 4.05 0 0 0 1.05-4.67Z',
}

/** Логотип провайдера; хост без известного типа — нейтральный сервер. Декоративный: имя хоста — в тексте рядом. */
export function ProviderIcon({ provider, className }: { provider: Provider | null; className?: string }) {
  if (!provider) return <ServerIcon aria-hidden className={cn('size-4 shrink-0', className)} />
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={cn('size-4 shrink-0', className)}>
      <path d={PATHS[provider]} />
    </svg>
  )
}
