import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react'
import { type DepDir, } from '../../../entities/node'
import { isLinkable, useReportView, visibleId } from '../../../entities/report'
import { cn } from '../../../shared/lib/cn'
import { pos } from '../../../shared/lib/timeline'

type Dir = DepDir
type Geometry = { left: number; width: number; height: number; paths: { d: string; dir: Dir }[] }

// короткий горизонтальный отрезок от конца полоски до вертикали
const STUB_PX = 6

type LinksOverlayProps = {
  host: RefObject<HTMLElement | null>
  /** дорожка оси: её ширина и левый край — те же, что у дорожек строк */
  track: RefObject<HTMLElement | null>
  rows: RefObject<Map<string, HTMLElement>>
}

/**
 * Линии связей выбранного узла: синие со стрелкой — от джоб, которых он ждал,
 * зелёные — к джобам, которые ждут его. Идут от конца полоски источника к началу полоски цели.
 */
export function LinksOverlay({ host, track, rows }: LinksOverlayProps) {
  const { tree, state, rel } = useReportView()
  const [geometry, setGeometry] = useState<Geometry | null>(null)

  const measure = () => {
    const hostEl = host.current
    const trackEl = track.current
    if (!hostEl || !trackEl || !rel.own.size) return setGeometry(null)
    const origin = hostEl.getBoundingClientRect()
    const box = trackEl.getBoundingClientRect()
    // строки лежат в карточках, поэтому координаты — от хоста, а не от offsetParent
    const centers = new Map<string, number>()
    for (const [id, el] of rows.current) {
      const r = el.getBoundingClientRect()
      centers.set(id, r.top - origin.top + r.height / 2)
    }
    const x = (ms: number) => (pos(ms, state.view) / 100) * box.width
    const paths: Geometry['paths'] = []
    const seen = new Set<string>()
    for (const [id, y2] of centers) {
      const target = tree.nodes[id]
      // у развёрнутой группы своей полоски нет: линии идут к её шардам
      if (!target?.bar || !isLinkable(target) || (target.kind === 'group' && !state.collapsed.has(id))) continue
      for (const dep of target.deps) {
        const srcId = visibleId(tree.nodes, state.collapsed, dep)
        const y1 = centers.get(srcId)
        const src = tree.nodes[srcId]
        if (y1 == null || !src?.bar || srcId === id || seen.has(`${srcId}>${id}`)) continue
        const dir: Dir | null = rel.own.has(id) ? 'up' : rel.own.has(dep) || rel.own.has(srcId) ? 'down' : null
        if (!dir) continue
        seen.add(`${srcId}>${id}`)
        const x1 = x(src.bar.end)
        paths.push({ dir, d: `M${x1},${y1} H${x1 + STUB_PX} V${y2} H${x(target.bar.start)}` })
      }
    }
    setGeometry({ left: box.left - origin.left, width: box.width, height: hostEl.scrollHeight, paths })
  }

  // измерение — после отрисовки строк: их положение и ширина дорожки известны только из DOM
  const latest = useRef(measure)
  useLayoutEffect(() => {
    latest.current = measure
    measure()
  }, [state.view, state.collapsed, rel, tree])

  // ресайз окна двигает дорожки без смены состояния
  useEffect(() => {
    const el = host.current
    if (!el) return
    const observer = new ResizeObserver(() => latest.current())
    observer.observe(el)
    return () => observer.disconnect()
  }, [host])

  if (!geometry) return null
  return (
    <svg aria-hidden className="pointer-events-none absolute top-0 overflow-hidden" style={{ left: geometry.left, width: geometry.width, height: geometry.height }}>
      <defs>
        {(['up', 'down'] as const).map((dir) => (
          <marker key={dir} id={`arrow-${dir}`} viewBox="0 0 6 6" refX="6" refY="3" markerWidth="6" markerHeight="6" markerUnits="userSpaceOnUse" orient="auto">
            <path d="M0,0 L6,3 L0,6 z" className={dir === 'up' ? 'fill-dep-up' : 'fill-dep-down'} />
          </marker>
        ))}
      </defs>
      {geometry.paths.map(({ d, dir }) => (
        <path key={d} d={d} markerEnd={`url(#arrow-${dir})`} className={cn('fill-none stroke-[1.5]', dir === 'up' ? 'stroke-dep-up' : 'stroke-dep-down')} />
      ))}
    </svg>
  )
}
