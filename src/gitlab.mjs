import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

const run = promisify(execFile)
const MAX_PARALLEL = 4
const MAX_DOWNSTREAM_DEPTH = 3
const MAX_LIST_PAGES = 10
const MAX_JOB_PAGES = 50

// glab может быть не установлен: ошибка значит «переходим к следующему источнику»
async function glabValue(exec, args) {
  try {
    const { stdout } = await exec('glab', args)
    return stdout.trim() || null
  } catch {
    return null
  }
}

function normalizeHost(v) {
  if (!v) return null
  const normalized = v.replace(/^https?:\/\//, '').replace(/\/$/, '')
  return normalized || null
}

export async function resolveHost(explicit, { exec = run } = {}) {
  const host = explicit ?? (await glabValue(exec, ['config', 'get', 'host']))
  if (!host) throw new Error('Не удалось определить хост GitLab: передай --host или ссылку на пайплайн')
  if (!/^[a-z0-9]([a-z0-9.-]*[a-z0-9])?(?::\d+)?$/i.test(host)) throw new Error(`Некорректный хост GitLab: ${host}`)
  return host
}

export async function resolveToken(host, { env = process.env, exec = run } = {}) {
  const glabToken = await glabValue(exec, ['config', 'get', 'token', '--host', host])
  if (glabToken) return glabToken

  if (env.GITLAB_TOKEN) {
    const glabHost = await glabValue(exec, ['config', 'get', 'host'])
    const normalizedEnvHost = normalizeHost(env.GITLAB_HOST)
    const normalizedGlabHost = normalizeHost(glabHost)
    if (normalizeHost(host) === normalizedEnvHost || normalizeHost(host) === normalizedGlabHost) {
      return env.GITLAB_TOKEN
    }
  }

  throw new Error(`Нет токена для ${host}. Выполни \`glab auth login --hostname ${host}\` или задай GITLAB_TOKEN вместе с GITLAB_HOST=${host}`)
}

export function createClient({ host, token, fetch = globalThis.fetch }) {
  let active = 0
  const waiting = []
  const acquire = () => (active < MAX_PARALLEL ? (active++, Promise.resolve()) : new Promise((r) => waiting.push(r)))
  const release = () => {
    const next = waiting.shift()
    if (next) next()
    else active--
  }

  const gql = async (query, variables = {}) => {
    await acquire()
    try {
      const res = await fetch(`https://${host}/api/graphql`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
        body: JSON.stringify({ query, variables }),
      })
      if (res.status === 401 || res.status === 403) {
        throw new Error(`GitLab ${host} ответил ${res.status}: токен недействителен или нет доступа`)
      }
      if (!res.ok) throw new Error(`GitLab ${host} ответил ${res.status}`)
      const body = await res.json()
      if (body.errors?.length) throw new Error(`GraphQL ${host}: ${body.errors.map((e) => e.message).join('; ')}`)
      return body.data
    } finally {
      release()
    }
  }
  gql.host = host
  return gql
}

const PIPELINE_QUERY = `query($project: ID!, $id: CiPipelineID!, $after: String) {
  project(fullPath: $project) {
    pipeline(id: $id) {
      id iid status createdAt finishedAt ref path
      jobs(first: 100, after: $after) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id name kind status startedAt finishedAt queuedDuration retried allowFailure webPath
          stage { name }
          previousStageJobsOrNeeds { nodes { ... on CiBuildNeed { name } ... on CiJob { name } } }
          downstreamPipeline { id project { fullPath } }
        }
      }
    }
  }
}`

export async function fetchPipeline(gql, project, id, depth = 0) {
  const jobs = []
  let pipeline = null
  let after = null
  let pageCount = 0
  do {
    if (pageCount >= MAX_JOB_PAGES) throw new Error(`Пайплайн ${id} в проекте ${project}: больше ${MAX_JOB_PAGES * 100} джоб, загрузка остановлена`)
    const data = await gql(PIPELINE_QUERY, { project, id, after })
    if (!data.project) throw new Error(`Проект ${project} на ${gql.host} не найден или нет доступа`)
    if (!data.project.pipeline) throw new Error(`Пайплайн ${id} в проекте ${project} на ${gql.host} не найден`)
    const { jobs: page, ...rest } = data.project.pipeline
    pipeline = rest
    jobs.push(...page.nodes)
    after = page.pageInfo.hasNextPage ? page.pageInfo.endCursor : null
    pageCount++
  } while (after)

  const downstream = {}
  if (depth < MAX_DOWNSTREAM_DEPTH) {
    await Promise.all(jobs.filter((j) => j.downstreamPipeline).map(async (j) => {
      const ds = j.downstreamPipeline
      downstream[j.id] = await fetchPipeline(gql, ds.project.fullPath, ds.id, depth + 1)
    }))
  }
  return { project, pipeline, jobs, downstream }
}

const LIST_QUERY = `query($project: ID!, $ref: String, $source: String, $after: String) {
  project(fullPath: $project) {
    pipelines(ref: $ref, source: $source, first: 100, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes { id status }
    }
  }
}`

export async function listPipelines(gql, project, { ref, source, statuses, last }) {
  const ids = []
  const counts = {}
  let after = null
  for (let page = 0; page < MAX_LIST_PAGES && ids.length < last; page++) {
    const data = await gql(LIST_QUERY, { project, ref, source, after })
    if (!data.project) throw new Error(`Проект ${project} на ${gql.host} не найден или нет доступа`)
    const { nodes, pageInfo } = data.project.pipelines
    for (const n of nodes) {
      if (ids.length >= last) break
      if (statuses && !statuses.includes(n.status)) continue
      ids.push(n.id)
      counts[n.status] = (counts[n.status] ?? 0) + 1
    }
    if (!pageInfo.hasNextPage) break
    after = pageInfo.endCursor
  }
  return { ids, counts }
}

const MR_QUERY = `query($project: ID!, $iid: String!) {
  project(fullPath: $project) { mergeRequest(iid: $iid) { headPipeline { id } } }
}`

export async function mrHeadPipeline(gql, project, iid) {
  const data = await gql(MR_QUERY, { project, iid })
  if (!data.project) throw new Error(`Проект ${project} на ${gql.host} не найден или нет доступа`)
  const id = data.project.mergeRequest?.headPipeline?.id
  if (!id) throw new Error(`У MR !${iid} в проекте ${project} нет пайплайна`)
  return id
}
