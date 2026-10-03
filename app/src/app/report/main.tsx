import { createRoot } from 'react-dom/client'
import './index.css'

// Данные подставляет Rust (core::render); в шаблоне блок пустой.
const data: unknown = JSON.parse(document.getElementById('data')!.textContent || 'null')

createRoot(document.getElementById('root')!).render(<pre>{JSON.stringify(data, null, 2)}</pre>)
