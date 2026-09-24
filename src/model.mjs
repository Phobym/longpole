const SHARD_RE = /(?:\s+\d+\/\d+|:\s*\[[^\]]*\])$/

export function shardGroupName(name) {
  return name.replace(SHARD_RE, '')
}

const orderKey = (s) => s.start ?? Infinity
export const byStart = (a, b) => (orderKey(a) === orderKey(b) ? 0 : orderKey(a) < orderKey(b) ? -1 : 1)

function bounds(spans) {
  let start = null
  let end = null
  for (const s of spans) {
    if (s.start != null && (start == null || s.start < start)) start = s.start
    if (s.end != null && (end == null || s.end > end)) end = s.end
  }
  return { start, end }
}

function span(fields) {
  return { queued: null, status: null, url: null, allowFailure: false, attempts: [], deps: [], children: [], after: null, ...fields }
}

const leafIds = (s) => (s.kind === 'group' ? s.children.flatMap(leafIds) : [s.id])

export function orderByDeps(children) {
  const sorted = [...children].sort(byStart)
  const rank = new Map(sorted.map((s, i) => [s, i]))
  // зависимость может указывать на шард: владелец шарда — его группа среди соседей
  const owner = new Map()
  for (const s of sorted) for (const id of [s.id, ...leafIds(s)]) owner.set(id, s)

  const parentOf = new Map()
  for (const s of sorted) {
    let parent = null
    for (const id of s.deps) {
      const o = owner.get(id)
      if (!o || o === s) continue
      const later = (o.end ?? -Infinity) > (parent?.end ?? -Infinity)
      const tieEarlier = parent && (o.end ?? -Infinity) === (parent.end ?? -Infinity) && rank.get(o) < rank.get(parent)
      if (parent === null || later || tieEarlier) parent = o
    }
    parentOf.set(s, parent)
  }

  const kids = new Map(sorted.map((s) => [s, []]))
  const roots = []
  for (const s of sorted) (parentOf.get(s) ? kids.get(parentOf.get(s)) : roots).push(s)

  const out = []
  const seen = new Set()
  const visit = (s) => {
    if (seen.has(s)) return
    seen.add(s)
    out.push(s)
    kids.get(s).forEach(visit)
  }
  roots.forEach(visit)
  // узлы из цикла зависимостей не достижимы от корней
  sorted.forEach(visit)
  for (const s of out) s.after = parentOf.get(s)?.id ?? null
  return out
}

export function buildTree(raw, { baseUrl, origin = Date.parse(raw.pipeline.createdAt), now = Date.now() }) {
  const at = (iso) => (iso == null ? null : Date.parse(iso) - origin)
  const endOf = (j) => (j.startedAt == null ? null : j.finishedAt != null ? at(j.finishedAt) : now - origin)
  const ordered = [...raw.jobs].reverse()
  const current = ordered.filter((j) => !j.retried)
  const idByName = new Map(current.map((j) => [j.name, j.id]))

  const jobs = current.map((j) => {
    const ds = raw.downstream[j.id]
    const children = ds ? [buildTree(ds, { baseUrl, origin, now })] : []
    const ownEnd = endOf(j)
    const dsEnd = children[0]?.end ?? null
    return span({
      id: j.id,
      kind: j.kind === 'BRIDGE' ? 'bridge' : 'job',
      name: j.name,
      stage: j.stage.name,
      start: at(j.startedAt),
      end: dsEnd == null ? ownEnd : Math.max(ownEnd ?? dsEnd, dsEnd),
      queued: j.queuedDuration == null ? null : Math.round(j.queuedDuration * 1000),
      status: j.status.toLowerCase(),
      allowFailure: j.allowFailure,
      url: baseUrl + j.webPath,
      attempts: ordered
        .filter((r) => r.retried && r.name === j.name)
        .map((r) => ({ start: at(r.startedAt), end: endOf(r), status: r.status.toLowerCase(), url: baseUrl + r.webPath })),
      deps: j.previousStageJobsOrNeeds.nodes.map((n) => idByName.get(n.name)).filter(Boolean),
      children,
    })
  })

  const p = raw.pipeline
  const stageNames = [...new Set(ordered.map((j) => j.stage.name))]
  const stages = stageNames.map((stage) => {
    const members = jobs.filter((s) => s.stage === stage).sort(byStart)
    const children = []
    const seenGroups = new Set()
    for (const s of members) {
      const group = shardGroupName(s.name)
      const shards = members.filter((x) => shardGroupName(x.name) === group)
      if (shards.length < 2) {
        children.push(s)
      } else if (!seenGroups.has(group)) {
        seenGroups.add(group)
        children.push(span({
          id: `${p.id}:${stage}:${group}`, kind: 'group', name: group, ...bounds(shards),
          deps: [...new Set(shards.flatMap((x) => x.deps))],
          children: orderByDeps(shards),
        }))
      }
    }
    return span({ id: `${p.id}:${stage}`, kind: 'stage', name: stage, ...bounds(children), children: orderByDeps(children) })
  })

  const stagesEnd = bounds(stages).end
  const finished = at(p.finishedAt)
  return span({
    id: p.id,
    kind: 'pipeline',
    name: `#${p.iid}`,
    project: raw.project,
    ref: p.ref,
    start: at(p.createdAt),
    end: finished == null ? stagesEnd : Math.max(finished, stagesEnd ?? finished),
    status: p.status.toLowerCase(),
    url: baseUrl + p.path,
    children: stages,
  })
}
