import { useCallback, useEffect, useRef, type MouseEvent } from 'react'
import { useReportView } from '../../../entities/report'
import { clampView, type View } from '../../../shared/lib/timeline'
import { trackOf } from './lane'
import './dragging.css'

const DRAG_THRESHOLD_PX = 4
// отпускание кнопки после перетаскивания порождает click: его нужно погасить
const CLICK_GRACE_MS = 50

type Drag = { x: number; view: View; width: number; moved: boolean }

/** Перетаскивание дорожки левой кнопкой: ось едет за мышью. */
export function useViewDrag() {
  const { getView, setView } = useReportView()
  const drag = useRef<Drag | null>(null)
  const endedAt = useRef(0)

  useEffect(() => {
    const end = () => {
      if (drag.current?.moved) endedAt.current = Date.now()
      drag.current = null
      document.body.classList.remove('dragging')
    }
    const move = (e: globalThis.MouseEvent) => {
      const d = drag.current
      if (!d) return
      if ((e.buttons & 1) === 0) return end()
      const dx = e.clientX - d.x
      if (!d.moved && Math.abs(dx) < DRAG_THRESHOLD_PX) return
      d.moved = true
      document.body.classList.add('dragging')
      const [a, b] = d.view
      const shift = (-dx / d.width) * (b - a)
      setView(clampView([a + shift, b + shift]))
    }
    document.addEventListener('mousemove', move)
    document.addEventListener('mouseup', end)
    return () => {
      document.removeEventListener('mousemove', move)
      document.removeEventListener('mouseup', end)
      end()
    }
  }, [setView])

  const onMouseDown = useCallback(
    (e: MouseEvent) => {
      const track = trackOf(e.target)
      if (!track || e.button !== 0) return
      e.preventDefault()
      drag.current = { x: e.clientX, view: getView(), width: track.getBoundingClientRect().width, moved: false }
    },
    [getView],
  )

  const justDragged = useCallback(() => Date.now() - endedAt.current < CLICK_GRACE_MS, [])

  return { onMouseDown, justDragged }
}
