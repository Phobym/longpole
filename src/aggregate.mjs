import { criticalPath } from './critical-path.mjs'
import { retryLoss } from './insights.mjs'
import { byStart, orderByDeps } from './model.mjs'

export function percentile(values, p) {
  if (values.length === 0) return null
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)]
}

const stat = (values) => ({ p50: percentile(values, 50), p90: percentile(values, 90) })
const keyOf = (s) => (s.kind === 'pipeline' ? 'pipeline' : `${s.kind}:${s.name}`)
const LINKED = new Set(['job', 'bridge', 'group'])
const ORDERED_BY_DEPS = new Set(['stage', 'group'])

export function aggregate(trees) {
  const critical = trees.map((t) => new Set(criticalPath(t).ids))
  // id исходного спана в пайплайне → id агрегированного узла; нужен, чтобы перевести deps
  const aggIdOf = new Map()
  const linked = []

  // entries — один и тот же узел в разных пайплайнах: [{ span, tree }]
  const merge = (id, kind, name, entries) => {
    for (const e of entries) aggIdOf.set(`${e.tree}|${e.span.id}`, id)
    const groups = new Map()
    for (const e of entries) {
      for (const c of e.span.children) {
        const key = keyOf(c)
        if (!groups.has(key)) groups.set(key, { kind: c.kind, name: c.name, entries: [] })
        groups.get(key).entries.push({ span: c, tree: e.tree })
      }
    }
    const children = [...groups]
      .map(([key, g]) => merge(`${id}/${key}`, g.kind, g.name, g.entries))
      // стейджи сортируем по объявленному в пайплайне порядку, а не по p50-старту (тот же баг, что в model.mjs)
      .sort((a, b) => (a.kind === 'stage' && b.kind === 'stage' ? a.position - b.position : byStart(a, b)))

    const ran = entries.filter((e) => e.span.start != null && e.span.end != null)
    const stats = {
      present: ran.length,
      total: trees.length,
      start: stat(ran.map((e) => e.span.start)),
      end: stat(ran.map((e) => e.span.end)),
      duration: stat(ran.map((e) => e.span.end - e.span.start)),
      queued: stat(ran.map((e) => e.span.queued).filter((q) => q != null)),
      retries: ran.length ? ran.reduce((n, e) => n + e.span.attempts.length, 0) / ran.length : 0,
      retried: ran.filter((e) => e.span.attempts.length > 0).length,
      retryLoss: ran.length ? ran.reduce((n, e) => n + retryLoss(e.span), 0) / ran.length : 0,
      critical: ran.filter((e) => critical[e.tree].has(e.span.id)).length / trees.length,
      samples: ran.map((e) => ({ tree: e.tree, start: e.span.start, end: e.span.end, retries: e.span.attempts.length })),
    }
    const node = {
      id, kind, name,
      start: stats.start.p50, end: stats.end.p50, queued: stats.queued.p50,
      status: null, url: null, allowFailure: false, attempts: [], deps: [], after: null,
      children, stats,
    }
    if (kind === 'stage') node.position = Math.min(...entries.map((e) => e.span.position))
    if (LINKED.has(kind)) linked.push({ node, entries })
    return node
  }

  const root = merge('agg', 'pipeline', `${trees.length} пайплайнов`, trees.map((span, tree) => ({ span, tree })))

  for (const { node, entries } of linked) {
    const deps = new Set()
    for (const e of entries) {
      for (const dep of e.span.deps) {
        const aggId = aggIdOf.get(`${e.tree}|${dep}`)
        if (aggId && aggId !== node.id) deps.add(aggId)
      }
    }
    node.deps = [...deps]
  }
  const reorder = (s) => {
    if (ORDERED_BY_DEPS.has(s.kind)) s.children = orderByDeps(s.children)
    s.children.forEach(reorder)
  }
  reorder(root)
  return root
}
