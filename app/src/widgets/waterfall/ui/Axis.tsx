import { duration } from '../../../shared/lib/format'
import { pos, tickStep, type View } from '../../../shared/lib/timeline'
import { GRID } from './grid'

const MAX_TICKS = 10

/** Ось времени от создания пайплайна; шаг мельчает при приближении. */
export function Axis({ view }: { view: View }) {
  const [a, b] = view
  const step = tickStep(b - a, MAX_TICKS)
  const ticks = []
  for (let t = Math.ceil(a / step) * step; t <= b; t += step) {
    ticks.push(
      <span key={t} className="absolute top-0 -translate-x-1/2 text-[10.5px] whitespace-nowrap text-muted-foreground" style={{ left: `${pos(t, view)}%` }}>
        {duration(t)}
      </span>,
    )
  }
  return (
    <div className={GRID}>
      <div />
      <div className="relative h-4 overflow-hidden">
        <div className="absolute inset-y-0 right-6 left-3">
          {ticks}
        </div>
      </div>
      <div />
    </div>
  )
}
