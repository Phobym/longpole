import type { Theme } from '../api/schema/Theme'

const media = window.matchMedia('(prefers-color-scheme: dark)')
let detach: (() => void) | null = null

/** Класс `dark` на `<html>`; `system` следит за системной темой до следующего вызова. */
export function applyTheme(theme: Theme) {
  detach?.()
  detach = null
  const set = (dark: boolean) => document.documentElement.classList.toggle('dark', dark)
  if (theme !== 'system') {
    set(theme === 'dark')
    return
  }
  set(media.matches)
  const onChange = (e: MediaQueryListEvent) => set(e.matches)
  media.addEventListener('change', onChange)
  detach = () => media.removeEventListener('change', onChange)
}
