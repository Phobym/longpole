import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

const root = document.getElementById('root')
if (!root) throw new Error('нет #root в index.html')

createRoot(root).render(
  <StrictMode>
    <h1>pipeline-trace</h1>
  </StrictMode>,
)
