import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Selfhosted в dev ходит на настоящий сервер (`url: http://localhost:5173` в его конфиге — тогда сходятся
// `Origin` и redirect URI); Desktop — на мок моста, прокси ему не мешает.
const server = 'http://localhost:8080'

// Форма → dist/ (frontendDist Tauri).
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 5173, strictPort: true, proxy: { '/api': server, '/auth': server, '/healthz': server } },
})
