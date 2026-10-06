import { useEffect } from 'react'
import { useReportView } from '../../../entities/report'
import { clampView } from '../../../shared/lib/timeline'

const KEY_SHIFT = 0.1

/** Клавиши отчёта: `[` и `]` сдвигают ось на 10% окна, `0` — весь пайплайн, Esc снимает выделение. */
export function useReportKeys() {
  const { getView, setView, fullView, dispatch, state } = useReportView()
  const selected = state.selected !== null

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return
      // Esc снимает выделение независимо от `defaultPrevented`: подсказка «?» (Radix) гасит событие и закрывается сама, а одного нажатия хватает на оба.
      // При выделении гасим событие сами, чтобы тот же Esc не увёл с отчёта (корневой обработчик в router.tsx); без выделения — пусть уходит назад
      if (e.key === 'Escape') {
        dispatch({ type: 'close' })
        if (selected) e.preventDefault()
        return
      }
      // строка могла обработать клавишу сама (стрелки, Enter)
      if (e.defaultPrevented) return
      if (e.key === '0') {
        setView(fullView)
      } else if (e.key === '[' || e.key === ']') {
        const [a, b] = getView()
        const d = (b - a) * (e.key === ']' ? KEY_SHIFT : -KEY_SHIFT)
        setView(clampView([a + d, b + d]))
      }
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [getView, setView, fullView, dispatch, selected])
}
