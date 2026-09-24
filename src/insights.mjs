import { criticalPath } from './critical-path.mjs'

export const MIN_SAVING_MS = 10_000
const MAX_HOTSPOTS = 3

const walk = (s, fn) => {
  fn(s)
  s.children.forEach((c) => walk(c, fn))
}
const leaves = (s) => (s.kind === 'job' || s.kind === 'bridge' ? [s] : s.children.flatMap(leaves))
const rank = (candidates) =>
  candidates.filter((c) => c.saving >= MIN_SAVING_MS).sort((a, b) => b.saving - a.saving).slice(0, MAX_HOTSPOTS)
const sum = (list) => list.reduce((n, c) => n + c.saving, 0)

// Столько пайплайн ждал из-за упавших попыток джобы
export function retryLoss(span) {
  const starts = span.attempts.map((a) => a.start).filter((v) => v != null)
  if (starts.length === 0 || span.start == null) return 0
  return Math.max(0, span.start - Math.min(...starts))
}

export function stageExcess(stage, endOf = (s) => s.end) {
  const ran = leaves(stage).filter((s) => endOf(s) != null).sort((a, b) => endOf(b) - endOf(a))
  if (ran.length < 2) return null
  const excess = endOf(ran[0]) - endOf(ran[1])
  return excess > 0 ? { id: ran[0].id, peersEnd: endOf(ran[1]), excess } : null
}

export function insights(tree) {
  const crit = new Set(criticalPath(tree).ids)
  const index = new Map()
  walk(tree, (s) => index.set(s.id, s))
  const stages = {}
  const losses = {}
  const candidates = []
  let totalRetryLoss = 0

  walk(tree, (s) => {
    if (s.kind === 'job' || s.kind === 'bridge') {
      const loss = retryLoss(s)
      if (loss === 0) return
      losses[s.id] = loss
      totalRetryLoss += loss
      if (crit.has(s.id)) candidates.push({ id: s.id, name: s.name, kind: 'retry', saving: loss, retries: s.attempts.length })
    } else if (s.kind === 'stage') {
      const ex = stageExcess(s)
      if (!ex) return
      stages[s.id] = ex
      if (!crit.has(ex.id)) return
      const bottleneck = index.get(ex.id)
      // ретраи узкого места уже стали отдельным пунктом — вычитаем, чтобы не считать дважды
      const saving = Math.max(0, ex.excess - retryLoss(bottleneck))
      candidates.push({ id: ex.id, name: bottleneck.name, kind: 'excess', saving, stage: s.name, excess: ex.excess })
    }
  })

  const hotspots = rank(candidates)
  return { stages, retryLoss: losses, totalRetryLoss, hotspots, saving: sum(hotspots) }
}
