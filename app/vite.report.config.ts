import { createHash } from 'node:crypto'
import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { viteSingleFile } from 'vite-plugin-singlefile'

const sha = (s: string) => `'sha256-${createHash('sha256').update(s, 'utf8').digest('base64')}'`

// После инлайна: sha256 каждого исполняемого <script> и <style> → <meta> CSP первым элементом <head>.
// JSON-блок данных не исполняется и не хешируется: Rust подставляет в него данные после сборки.
// ponytail: теги ищутся регулярками — годится для вывода vite-plugin-singlefile (атрибуты без `>`);
// если шаблон начнёт содержать такие атрибуты или вложенные теги, перейти на HTML-парсер.
function cspHashes(): Plugin {
  return {
    name: 'csp-hashes',
    enforce: 'post',
    generateBundle: {
      order: 'post',
      handler(_, bundle) {
        const html = bundle['report.html']
        if (html?.type !== 'asset' || typeof html.source !== 'string') this.error('нет report.html в бандле')
        let src = html.source
        const scripts = [...src.matchAll(/<script(?![^>]*type="application\/json")[^>]*>([\s\S]*?)<\/script>/g)].map((m) => sha(m[1]))
        const styles = [...src.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => sha(m[1]))
        const csp = `default-src 'none'; script-src ${scripts.join(' ')}; style-src ${styles.join(' ')}; img-src data:; connect-src 'none'`
        src = src.replace('<head>', () => `<head>\n<meta http-equiv="Content-Security-Policy" content="${csp}">`)
        html.source = src
      },
    },
  }
}

// Отчёт → dist-report/report.html, один файл; Rust встраивает его как шаблон.
export default defineConfig({
  plugins: [react(), tailwindcss(), viteSingleFile({ removeViteModuleLoader: true }), cspHashes()],
  build: {
    outDir: 'dist-report',
    rolldownOptions: { input: 'report.html' },
  },
})
