import i18next from 'i18next'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { getLocale, type Locale } from '../../shared/api'
import { initI18n } from '../../shared/i18n'
import '../../shared/ui/theme.css'
import { router } from './router'

// Ошибки показывает сама форма, повторы только задержали бы их; окно — не вкладка, перезапрос по фокусу не нужен.
const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, staleTime: 60_000 } },
})

// Вне Tauri (vite в браузере) `get_locale` недоступна — тогда язык браузера.
const browserLocale = (): Locale => (navigator.language.toLowerCase().startsWith('ru') ? 'ru' : 'en')

async function main() {
  const root = document.getElementById('root')
  if (!root) throw new Error('нет #root в index.html')
  const locale = await getLocale()
  const initial = locale.ok ? locale.value : browserLocale()
  await initI18n(initial)
  document.documentElement.lang = initial
  i18next.on('languageChanged', (lng) => {
    document.documentElement.lang = lng
  })
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
