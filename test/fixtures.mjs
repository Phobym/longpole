export const T0 = '2026-09-24T10:00:00+03:00'

export const at = (sec) => new Date(Date.parse(T0) + sec * 1000).toISOString()

export function job(name, stage, {
  start = null, end = null, deps = [], retried = false, status = 'SUCCESS', kind = 'BUILD',
  queued = 1, allowFailure = false, downstream = null,
  id = `gid://gitlab/Ci::Build/${name}${retried ? `-r${start}` : ''}`,
} = {}) {
  return {
    id, name, kind, status,
    startedAt: start == null ? null : at(start),
    finishedAt: end == null ? null : at(end),
    queuedDuration: start == null ? null : queued,
    retried, allowFailure,
    webPath: `/g/p/-/jobs/${encodeURIComponent(name)}`,
    stage: { name: stage },
    previousStageJobsOrNeeds: { nodes: deps.map((n) => ({ name: n })) },
    downstreamPipeline: downstream,
  }
}

// jobs перечисляются в хронологическом порядке, API отдаёт их от новых к старым
export function rawPipeline(jobs, {
  id = 'gid://gitlab/Ci::Pipeline/1', iid = '1', status = 'SUCCESS', createdAt = T0,
  finishedAt = null, project = 'g/p', downstream = {}, stages,
} = {}) {
  return {
    project,
    pipeline: {
      id, iid, status, createdAt, finishedAt, ref: 'master', path: `/${project}/-/pipelines/${iid}`,
      ...(stages ? { stages: { nodes: stages.map((name) => ({ name })) } } : {}),
    },
    jobs: [...jobs].reverse(),
    downstream,
  }
}
