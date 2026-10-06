import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { settingsQuery } from '../../entities/settings'
import { getSettings, type Locale } from '../../shared/api'
import { initI18n } from '../../shared/i18n'
import { applyTheme } from '../../shared/lib/theme'
import '../../shared/ui/theme.css'
import { router } from './router'

// Ошибки показывает сама форма, повторы только задержали бы их; окно — не вкладка, перезапрос по фокусу не нужен.
const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, staleTime: 60_000 } },
})

// Вне Tauri (vite в браузере) `get_settings` недоступна — тогда язык браузера.
const browserLocale = (): Locale => (navigator.language.toLowerCase().startsWith('ru') ? 'ru' : 'en')

async function main() {
  // в браузере без Tauri — мок моста, иначе `get_settings` падает на `invoke`
  if (import.meta.env.DEV) await import('../dev/mock')
  const root = document.getElementById('root')
  if (!root) throw new Error('нет #root в index.html')
  const settings = await getSettings()
  const initial = settings.ok ? settings.value.locale : browserLocale()
  applyTheme(settings.ok ? settings.value.theme : 'system')
  if (settings.ok) queryClient.setQueryData(settingsQuery.queryKey, settings.value)
  await initI18n(initial)
  createRoot(root).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  )
}

main().catch((e) => {
  document.body.textContent = String(e)
})
