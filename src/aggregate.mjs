import { criticalPath } from './critical-path.mjs'

export function percentile(values, p) {
  if (values.length === 0) return null
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)]
}

const stat = (values) => ({ p50: percentile(values, 50), p90: percentile(values, 90) })
const keyOf = (s) => (s.kind === 'pipeline' ? 'pipeline' : `${s.kind}:${s.name}`)
const orderKey = (s) => s.start ?? Infinity
const byStart = (a, b) => (orderKey(a) === orderKey(b) ? 0 : orderKey(a) < orderKey(b) ? -1 : 1)

export function aggregate(trees) {
  const critical = trees.map((t) => new Set(criticalPath(t).ids))

  // entries — один и тот же узел в разных пайплайнах: [{ span, tree }]
  const merge = (id, kind, name, entries) => {
    const groups = new Map()
    for (const e of entries) {
      for (const c of e.span.children) {
        const key = keyOf(c)
        if (!groups.has(key)) groups.set(key, { kind: c.kind, name: c.name, entries: [] })
        groups.get(key).entries.push({ span: c, tree: e.tree })
      }
    }
    const children = [...groups].map(([key, g]) => merge(`${id}/${key}`, g.kind, g.name, g.entries)).sort(byStart)

    const ran = entries.filter((e) => e.span.start != null && e.span.end != null)
    const stats = {
      present: ran.length,
      total: trees.length,
      start: stat(ran.map((e) => e.span.start)),
      end: stat(ran.map((e) => e.span.end)),
      duration: stat(ran.map((e) => e.span.end - e.span.start)),
      queued: stat(ran.map((e) => e.span.queued).filter((q) => q != null)),
      retries: ran.length ? ran.reduce((n, e) => n + e.span.attempts.length, 0) / ran.length : 0,
      critical: ran.filter((e) => critical[e.tree].has(e.span.id)).length / trees.length,
      samples: ran.map((e) => ({ tree: e.tree, start: e.span.start, end: e.span.end })),
    }
    return {
      id, kind, name,
      start: stats.start.p50, end: stats.end.p50, queued: stats.queued.p50,
      status: null, url: null, allowFailure: false, attempts: [], deps: [],
      children, stats,
    }
  }

  return merge('agg', 'pipeline', `${trees.length} пайплайнов`, trees.map((span, tree) => ({ span, tree })))
}
