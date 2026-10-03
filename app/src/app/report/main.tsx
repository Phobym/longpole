import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'

const root = document.getElementById('root')
if (!root) throw new Error('нет #root в report.html')

// Данные подставляет Rust (core::render); в шаблоне блок пустой.
let data: unknown
try {
  data = JSON.parse(document.getElementById('data')?.textContent || 'null')
} catch (e) {
  data = `данные отчёта повреждены: ${String(e)}`
}

createRoot(root).render(
  <StrictMode>
    <pre>{JSON.stringify(data, null, 2)}</pre>
  </StrictMode>,
)
