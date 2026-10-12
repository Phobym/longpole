import { existsSync, readFileSync } from 'node:fs'
import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Selfhosted в dev ходит на настоящий сервер (`url: http://localhost:5173` в его конфиге — тогда сходятся
// `Origin` и redirect URI); Desktop — на мок моста, прокси ему не мешает.
const server = 'http://localhost:8080'

const TEMPLATE = 'dist-report/report.html'

// «Скачать HTML» в Selfhosted (спека поставки, § 10): шаблон отчёта — в статике текстом, не страницей, чтобы
// самодостаточный HTML не открывался на origin сервера. В Desktop шаблон встраивает ядро, тут его нет.
// `npm run build` собирает шаблон первым; без него сборка на диск падает, сборка в память (тест до `build`) — нет.
// ponytail: в `vite dev` файла нет — «Скачать HTML» работает только со сборкой
const reportTemplate = (): Plugin => {
  let write = true
  return {
    name: 'report-template',
    apply: 'build',
    configResolved(config) {
      write = config.build.write
    },
    generateBundle() {
      if (process.env.VITE_DELIVERY !== 'selfhosted') return
      if (!existsSync(TEMPLATE)) return write ? this.error(`нет ${TEMPLATE}: сначала vite build -c vite.report.config.ts`) : undefined
      this.emitFile({ type: 'asset', fileName: 'report-template.txt', source: readFileSync(TEMPLATE, 'utf8') })
    },
  }
}

// Форма → dist/ (frontendDist Tauri).
export default defineConfig({
  plugins: [react(), tailwindcss(), reportTemplate()],
  clearScreen: false,
  server: { port: 5173, strictPort: true, proxy: { '/api': server, '/auth': server, '/healthz': server } },
})
