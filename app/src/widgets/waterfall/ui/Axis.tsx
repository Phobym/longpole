import type { Ref } from 'react'
import { duration } from '../../../shared/lib/format'
import { pos, type View } from '../../../shared/lib/timeline'
import { GRID } from './grid'

const STEPS = [1e3, 5e3, 1e4, 3e4, 6e4, 12e4, 3e5, 6e5, 9e5, 18e5, 36e5, 72e5]
const MAX_TICKS = 10

/** Ось времени от создания пайплайна; шаг мельчает при приближении. */
export function Axis({ view, trackRef }: { view: View; trackRef: Ref<HTMLDivElement> }) {
  const [a, b] = view
  const step = STEPS.find((x) => (b - a) / x <= MAX_TICKS) ?? STEPS[STEPS.length - 1]
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
        <div ref={trackRef} className="absolute inset-y-0 right-6 left-3">
          {ticks}
        </div>
      </div>
      <div />
    </div>
  )
}
