import { fetchPipeline, listPipelines, mrHeadPipeline } from './gitlab.mjs'
import { buildTree } from './model.mjs'
import { aggregate } from './aggregate.mjs'
import { aggInsights, insights, stabilityByName } from './insights.mjs'

const slug = (s) => s.replace(/[^\w.-]+/g, '-')

export function defaultFileName(project, suffix) {
  return `pipeline-trace-${slug(project)}-${slug(suffix)}.html`
}

export async function buildReport(request, { gql, host, now = () => new Date(), onProgress = () => {} }) {
  let ids
  let statusCounts = null
  let suffix
  if (request.mode === 'pipeline') {
    ids = [`gid://gitlab/Ci::Pipeline/${request.pipelineId}`]
    suffix = request.pipelineId
  } else if (request.mode === 'mr') {
    ids = [await mrHeadPipeline(gql, request.project, request.mrIid)]
    suffix = `mr${request.mrIid}`
  } else {
    ;({ ids, counts: statusCounts } = await listPipelines(gql, request.project, request))
    if (ids.length === 0) throw new Error('Под фильтры не попал ни один пайплайн: проверь ref, source и статусы (--ref, --source, --status)')
    suffix = request.ref ?? request.source ?? 'all'
  }

  let loaded = 0
  onProgress({ loaded, total: ids.length })
  const raws = await Promise.all(ids.map(async (id) => {
    const raw = await fetchPipeline(gql, request.project, id)
    onProgress({ loaded: ++loaded, total: ids.length })
    return raw
  }))
  const trees = raws.map((raw) => buildTree(raw, { baseUrl: `https://${host}` }))
  const isAggregate = request.mode === 'aggregate'
  const agg = isAggregate ? aggregate(trees) : null
  const meta = {
    mode: isAggregate ? 'aggregate' : 'single',
    host,
    project: request.project,
    label: isAggregate ? [request.ref, request.source].filter(Boolean).join(' · ') || 'все пайплайны' : trees[0].name,
    statusCounts,
    generatedAt: now().toISOString(),
  }
  const report = {
    meta, trees, agg,
    insights: { trees: trees.map(insights), agg: agg && aggInsights(agg), stability: agg && stabilityByName(agg) },
  }
  return { report, suffix }
}
