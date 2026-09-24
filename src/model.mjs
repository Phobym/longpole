const SHARD_RE = /(?:\s+\d+\/\d+|:\s*\[[^\]]*\])$/

export function shardGroupName(name) {
  return name.replace(SHARD_RE, '')
}

const orderKey = (s) => s.start ?? Infinity
const byStart = (a, b) => (orderKey(a) === orderKey(b) ? 0 : orderKey(a) < orderKey(b) ? -1 : 1)

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
  return { queued: null, status: null, url: null, allowFailure: false, attempts: [], deps: [], children: [], ...fields }
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
        children.push(span({ id: `${p.id}:${stage}:${group}`, kind: 'group', name: group, ...bounds(shards), children: shards }))
      }
    }
    return span({ id: `${p.id}:${stage}`, kind: 'stage', name: stage, ...bounds(children), children })
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
