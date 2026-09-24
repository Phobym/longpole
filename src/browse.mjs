const PAGE = 20

const PROJECTS_QUERY = `query($search: String, $after: String) {
  projects(membership: true, search: $search, sort: "latest_activity_desc", first: ${PAGE}, after: $after) {
    pageInfo { hasNextPage endCursor }
    nodes { fullPath nameWithNamespace lastActivityAt repository { rootRef } }
  }
}`

const BRANCHES_QUERY = `query($project: ID!, $pattern: String!) {
  project(fullPath: $project) { repository { rootRef branchNames(searchPattern: $pattern, offset: 0, limit: ${PAGE}) } }
}`

const PIPELINES_QUERY = `query($project: ID!, $ref: String, $after: String) {
  project(fullPath: $project) {
    pipelines(ref: $ref, first: ${PAGE}, after: $after) {
      pageInfo { hasNextPage endCursor }
      nodes { id iid status source createdAt duration commit { shortId title } user { username } }
    }
  }
}`

const nextOf = (pageInfo) => (pageInfo.hasNextPage ? pageInfo.endCursor : null)
const notFound = (gql, project) => new Error(`Проект ${project} на ${gql.host} не найден или нет доступа`)

export async function listProjects(gql, { search = '', after = null } = {}) {
  const data = await gql(PROJECTS_QUERY, { search: search.trim() || null, after })
  const { nodes, pageInfo } = data.projects
  return {
    items: nodes.map((n) => ({ fullPath: n.fullPath, name: n.nameWithNamespace, lastActivityAt: n.lastActivityAt, defaultBranch: n.repository?.rootRef ?? null })),
    next: nextOf(pageInfo),
  }
}

export async function listBranches(gql, project, search = '') {
  const data = await gql(BRANCHES_QUERY, { project, pattern: `*${search.trim()}*`.replace('**', '*') })
  if (!data.project) throw notFound(gql, project)
  const { rootRef = null, branchNames = [] } = data.project.repository ?? {}
  const names = branchNames ?? []
  return rootRef && names.includes(rootRef) ? [rootRef, ...names.filter((n) => n !== rootRef)] : names
}

export async function recentPipelines(gql, project, { ref = null, after = null } = {}) {
  const data = await gql(PIPELINES_QUERY, { project, ref, after })
  if (!data.project) throw notFound(gql, project)
  const { nodes, pageInfo } = data.project.pipelines
  return {
    items: nodes.map((n) => ({
      id: n.id.split('/').pop(),
      iid: n.iid,
      status: n.status.toLowerCase(),
      source: n.source,
      createdAt: n.createdAt,
      duration: n.duration == null ? null : n.duration * 1000,
      commit: n.commit ? { sha: n.commit.shortId, title: n.commit.title } : null,
      author: n.user?.username ?? null,
    })),
    next: nextOf(pageInfo),
  }
}
