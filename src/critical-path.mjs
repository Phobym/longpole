export function criticalPath(root, scopeId = root.id) {
  const byId = new Map()
  const index = (s) => {
    byId.set(s.id, s)
    s.children.forEach(index)
  }
  index(root)

  const leaves = (s) => (s.kind === 'job' || s.kind === 'bridge' ? [s] : s.children.flatMap(leaves))
  const latest = (spans) =>
    spans.reduce((best, s) => (s && s.end != null && (best === null || s.end > best.end) ? s : best), null)

  const path = []
  const walk = (job) => {
    for (; job; job = latest(job.deps.map((id) => byId.get(id)))) {
      path.push(job)
      if (job.kind === 'bridge' && job.children[0]) walk(latest(leaves(job.children[0])))
    }
  }
  const scope = byId.get(scopeId)
  walk(latest(scope.kind === 'bridge' ? [scope] : leaves(scope)))
  path.reverse()

  const gaps = []
  for (let i = 1; i < path.length; i++) {
    const prev = path[i - 1]
    const next = path[i]
    if (next.start != null && next.start > prev.end) gaps.push({ from: prev.id, to: next.id, ms: next.start - prev.end })
  }
  return { ids: path.map((s) => s.id), gaps }
}
