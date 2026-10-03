import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Форма → dist/ (frontendDist Tauri).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
})
