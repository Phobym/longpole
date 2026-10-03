import { useEffect, type RefObject } from 'react'
import { useReportView } from '../../../entities/report'
import { clampView } from '../../../shared/lib/timeline'
import { trackOf } from './lane'

const MIN_VIEW_MS = 1000
const ZOOM_SPEED = 0.004
const LINE_HEIGHT_PX = 16

/**
 * Колесо над дорожкой: ⌘/Ctrl (и pinch — он приходит как wheel с ctrlKey) — масштаб вокруг курсора,
 * Shift и горизонтальный свайп — сдвиг. Обычное вертикальное колесо прокручивает страницу.
 */
export function useWheelZoom(host: RefObject<HTMLElement | null>) {
  const { getView, setView, fullView } = useReportView()

  useEffect(() => {
    const el = host.current
    if (!el) return
    const onWheel = (e: WheelEvent) => {
      const track = trackOf(e.target)
      const zoom = e.ctrlKey || e.metaKey
      const horizontal = Math.abs(e.deltaX) > Math.abs(e.deltaY)
      if (!track || !(zoom || e.shiftKey || horizontal)) return
      e.preventDefault()
      const unit = e.deltaMode === 1 ? LINE_HEIGHT_PX : 1
      const rect = track.getBoundingClientRect()
      const [a, b] = getView()
      const width = b - a
      if (zoom) {
        const f = (e.clientX - rect.left) / rect.width
        const next = Math.min(Math.max(width * Math.exp(e.deltaY * unit * ZOOM_SPEED), MIN_VIEW_MS), fullView[1])
        const left = a + f * width - f * next
        setView(clampView([left, left + next]))
      } else {
        const delta = ((horizontal ? e.deltaX : e.deltaY) * unit * width) / rect.width
        setView(clampView([a + delta, b + delta]))
      }
    }
    // passive: false — иначе preventDefault не отменит прокрутку страницы
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  }, [host, getView, setView, fullView])
}
