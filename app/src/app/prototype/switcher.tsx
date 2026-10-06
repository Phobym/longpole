/**
 * ПРОТОТИП: три варианта визуального языка поверх реальных экранов, переключаются `?variant=`
 * (`current` — как сейчас). Стили — в `variants.css` по `data-variant` на `<html>`. Только dev, в сборку не попадает.
 */
import { useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import './variants.css'

const VARIANTS = [
  ['current', 'как сейчас (shadcn по умолчанию)'],
  ['swiss', 'A · Инструментальный, Swiss'],
  ['mono', 'B · Терминальный, mono-first'],
  ['native', 'C · Нативный desktop'],
] as const
type Variant = (typeof VARIANTS)[number][0]

const read = (): Variant => {
  const v = new URLSearchParams(location.search).get('variant')
  return VARIANTS.some(([key]) => key === v) ? (v as Variant) : 'current'
}

const apply = (variant: Variant) => {
  if (variant === 'current') delete document.documentElement.dataset.variant
  else document.documentElement.dataset.variant = variant
  const url = new URL(location.href)
  if (variant === 'current') url.searchParams.delete('variant')
  else url.searchParams.set('variant', variant)
  history.replaceState(history.state, '', url)
}

function Switcher() {
  const [variant, setVariant] = useState<Variant>(read)
  useEffect(() => apply(variant), [variant])
  const index = VARIANTS.findIndex(([key]) => key === variant)
  const step = (d: number) => setVariant(VARIANTS[(index + d + VARIANTS.length) % VARIANTS.length][0])
  // Shift+←/→: голые стрелки заняты строками отчёта
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.shiftKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return
      e.preventDefault()
      step(e.key === 'ArrowLeft' ? -1 : 1)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })
  const dark = () => document.documentElement.classList.toggle('dark')
  const btn = { font: 'inherit', background: 'none', border: 0, color: 'inherit', cursor: 'pointer', padding: '2px 8px' }
  return (
    <div
      style={{
        position: 'fixed',
        bottom: 14,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 9999,
        display: 'flex',
        alignItems: 'center',
        gap: 4,
        padding: '4px 6px',
        borderRadius: 999,
        background: '#111',
        color: '#fff',
        font: '500 12px/1.4 system-ui, sans-serif',
        boxShadow: '0 6px 24px rgba(0,0,0,.35), 0 0 0 1px rgba(255,255,255,.15)',
      }}
    >
      <button type="button" style={btn} onClick={() => step(-1)} aria-label="предыдущий вариант">
        ←
      </button>
      <span style={{ minWidth: 260, textAlign: 'center' }}>
        {index + 1}/{VARIANTS.length} · {VARIANTS[index][1]}
      </span>
      <button type="button" style={btn} onClick={() => step(1)} aria-label="следующий вариант">
        →
      </button>
      <span style={{ width: 1, height: 16, background: 'rgba(255,255,255,.25)' }} />
      <button type="button" style={btn} onClick={dark} title="переключить тёмную тему">
        ☾
      </button>
    </div>
  )
}

export function mountSwitcher() {
  apply(read())
  const host = document.createElement('div')
  document.body.appendChild(host)
  createRoot(host).render(<Switcher />)
}
