# pipeline-trace: план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CLI-утилита `pipeline-trace`, которая по ссылке на GitLab-пайплайн или по фильтру пайплайнов проекта собирает один HTML-файл с водопадом стейджей и джоб, критическим путём и агрегатом p50/p90.

**Architecture:** Модуль `gitlab.mjs` загружает сырые пайплайны через GraphQL. Модуль `model.mjs` превращает их в дерево спанов. Модули `critical-path.mjs` и `aggregate.mjs` считают метрики. Модуль `render.mjs` встраивает данные и `critical-path.mjs` в `template.html`. Модули связаны только форматом спана; `aggregate.mjs` использует `criticalPath`.

**Tech Stack:** Node 22, ES-модули, `node:test`, `node:util` `parseArgs`, встроенный `fetch`. Runtime- и dev-зависимостей нет.

Спецификация: `docs/specs/2026-09-24-pipeline-trace-design.md`.

## Global Constraints

- `"engines": { "node": ">=22" }`, `"type": "module"`, без зависимостей в `package.json`.
- Бинарь: `"bin": { "pipeline-trace": "./bin/pipeline-trace.mjs" }`.
- Все времена спана — миллисекунды от `createdAt` корневого пайплайна.
- Токен: `glab config get token --host <host>`, затем `GITLAB_TOKEN` — только если хост совпадает с `GITLAB_HOST` или с хостом `glab` по умолчанию. Заголовок `Authorization: Bearer <token>`.
- GraphQL-эндпоинт: `https://<host>/api/graphql`. Идентификаторы пайплайнов — глобальные ID вида `gid://gitlab/Ci::Pipeline/<id>`; в URL пайплайна стоит числовой `id`, не `iid`.
- `--status` по умолчанию `success,manual`; `any` — без фильтра.
- Одновременно не больше 4 HTTP-запросов к GitLab.
- Тексты для пользователя (ошибки, интерфейс) — на русском.
- Комментарии в коде — только там, где причина неочевидна из кода.

## Формат данных (общий для всех задач)

Сырой пайплайн — результат `fetchPipeline`:

```js
{
  project: 'group/name',
  pipeline: { id: 'gid://gitlab/Ci::Pipeline/1', iid: '51216', status: 'MANUAL',
              createdAt: ISO, finishedAt: ISO | null, ref: 'master', path: '/group/name/-/pipelines/1' },
  jobs: [ // порядок API: от новых к старым
    { id: 'gid://gitlab/Ci::Build/9', name: 'build:server', kind: 'BUILD' | 'BRIDGE',
      status: 'SUCCESS', startedAt: ISO | null, finishedAt: ISO | null,
      queuedDuration: 2.25 /* секунды */ | null, retried: false, allowFailure: false,
      webPath: '/group/name/-/jobs/9', stage: { name: 'build' },
      previousStageJobsOrNeeds: { nodes: [{ name: 'cache:npm' }] },
      downstreamPipeline: { id: 'gid://…', project: { fullPath: 'other/proj' } } | null },
  ],
  downstream: { [bridgeJobId]: rawPipeline },
}
```

Спан — результат `buildTree`:

```js
{ id, kind: 'pipeline' | 'stage' | 'group' | 'job' | 'bridge', name,
  start: ms | null, end: ms | null, queued: ms | null,
  status: 'success' | … | null, url: string | null, allowFailure: boolean,
  attempts: [{ start, end, status, url }], deps: [spanId], children: [span],
  // только job/bridge: stage; только pipeline: project, ref
}
```

Агрегированный спан — тот же формат плюс `stats`:

```js
stats: { present, total,
  start: { p50, p90 }, end: { p50, p90 }, duration: { p50, p90 }, queued: { p50, p90 },
  retries /* среднее */, critical /* доля пайплайнов 0..1 */,
  samples: [{ tree /* индекс в trees */, start, end }] }
```

## Файлы

```
package.json
.gitignore
README.md
bin/pipeline-trace.mjs     оркестрация CLI
src/cli-args.mjs           разбор аргументов и ссылок
src/gitlab.mjs             токен, хост, GraphQL-клиент, загрузка пайплайнов
src/model.mjs              сырой пайплайн → дерево спанов
src/critical-path.mjs      критический путь, без импортов (встраивается в HTML)
src/aggregate.mjs          перцентили и агрегированное дерево
src/render.mjs             сборка HTML
src/template.html          интерфейс
test/fixtures.mjs          фабрики сырых джоб и пайплайнов
test/*.test.mjs
```

---

### Task 1: Каркас пакета и разбор аргументов

**Files:**
- Create: `package.json`, `.gitignore`, `src/cli-args.mjs`
- Test: `test/cli-args.test.mjs`

**Interfaces:**
- Produces: `parseCliArgs(argv: string[])` →
  - `{ mode: 'help', usage }`
  - `{ mode: 'pipeline', host, project, pipelineId, out, open }`
  - `{ mode: 'mr', host, project, mrIid, out, open }`
  - `{ mode: 'aggregate', host: string | null, project, ref: string | null, source: string | null, last: number, statuses: string[] | null, out, open }`
  - бросает `Error` с текстом использования при неверных аргументах.

- [ ] **Step 1: Создать `package.json` и `.gitignore`**

`package.json`:

```json
{
  "name": "pipeline-trace",
  "version": "0.1.0",
  "description": "Водопад длительности GitLab-пайплайнов в одном HTML-файле",
  "type": "module",
  "bin": { "pipeline-trace": "./bin/pipeline-trace.mjs" },
  "files": ["bin", "src"],
  "engines": { "node": ">=22" },
  "scripts": { "test": "node --test" }
}
```

`.gitignore`:

```
node_modules/
pipeline-trace-*.html
```

- [ ] **Step 2: Написать падающий тест**

`test/cli-args.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { parseCliArgs } from '../src/cli-args.mjs'

test('ссылка на пайплайн даёт режим pipeline с числовым id', () => {
  const args = parseCliArgs(['https://gitlab.litres.io/platform/react/monorepo/-/pipelines/1025721'])
  assert.deepEqual(args, {
    mode: 'pipeline', host: 'gitlab.litres.io', project: 'platform/react/monorepo',
    pipelineId: '1025721', out: null, open: true,
  })
})

test('ссылка на MR даёт режим mr, хвост ссылки игнорируется', () => {
  const args = parseCliArgs(['https://gitlab.litres.io/g/p/-/merge_requests/7632/pipelines', '--no-open'])
  assert.equal(args.mode, 'mr')
  assert.equal(args.mrIid, '7632')
  assert.equal(args.project, 'g/p')
  assert.equal(args.open, false)
})

test('агрегат: статусы по умолчанию success,manual в верхнем регистре', () => {
  const args = parseCliArgs(['--project', 'g/p', '--ref', 'master', '--last', '20'])
  assert.deepEqual(args, {
    mode: 'aggregate', host: null, project: 'g/p', ref: 'master', source: null,
    last: 20, statuses: ['SUCCESS', 'MANUAL'], out: null, open: true,
  })
})

test('агрегат: --status any снимает фильтр', () => {
  assert.equal(parseCliArgs(['--project', 'g/p', '--status', 'any']).statuses, null)
})

test('агрегат: --source и --host передаются как есть', () => {
  const args = parseCliArgs(['--project', 'g/p', '--source', 'merge_request_event', '--host', 'h.example'])
  assert.equal(args.source, 'merge_request_event')
  assert.equal(args.host, 'h.example')
  assert.equal(args.last, 50)
})

test('ошибки: нет ссылки и проекта, неверная ссылка, неверный --last', () => {
  assert.throws(() => parseCliArgs([]), /Нужна ссылка или --project/)
  assert.throws(() => parseCliArgs(['https://h/g/p/-/jobs/1']), /Не похоже на ссылку/)
  assert.throws(() => parseCliArgs(['--project', 'g/p', '--last', '0']), /--last/)
})

test('--help возвращает текст использования', () => {
  assert.match(parseCliArgs(['--help']).usage, /pipeline-trace <url/)
})
```

- [ ] **Step 3: Запустить тест и убедиться, что он падает**

Run: `node --test test/cli-args.test.mjs`
Expected: FAIL, `Cannot find module '…/src/cli-args.mjs'`.

- [ ] **Step 4: Реализовать `src/cli-args.mjs`**

```js
import { parseArgs } from 'node:util'

const USAGE = `Использование:
  pipeline-trace <url пайплайна или MR>
  pipeline-trace --project group/name [--ref master] [--source merge_request_event] [--last 50]
                 [--status success,manual|any] [--host gitlab.example.com]
Общие флаги: [--out файл.html] [--no-open]`

const URL_RE = /^https?:\/\/([^/]+)\/(.+?)\/-\/(pipelines|merge_requests)\/(\d+)/

export function parseCliArgs(argv) {
  const { values, positionals } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      project: { type: 'string' },
      ref: { type: 'string' },
      source: { type: 'string' },
      last: { type: 'string', default: '50' },
      status: { type: 'string', default: 'success,manual' },
      host: { type: 'string' },
      out: { type: 'string' },
      'no-open': { type: 'boolean', default: false },
      help: { type: 'boolean', default: false },
    },
  })
  if (values.help) return { mode: 'help', usage: USAGE }
  const common = { out: values.out ?? null, open: !values['no-open'] }

  if (positionals.length > 0) {
    const match = URL_RE.exec(positionals[0])
    if (!match) throw new Error(`Не похоже на ссылку на пайплайн или MR: ${positionals[0]}\n\n${USAGE}`)
    const [, host, project, kind, id] = match
    return kind === 'pipelines'
      ? { mode: 'pipeline', host, project, pipelineId: id, ...common }
      : { mode: 'mr', host, project, mrIid: id, ...common }
  }

  if (!values.project) throw new Error(`Нужна ссылка или --project\n\n${USAGE}`)
  const last = Number(values.last)
  if (!Number.isInteger(last) || last < 1) throw new Error(`--last должно быть целым числом больше 0: ${values.last}`)
  const statuses = values.status === 'any'
    ? null
    : values.status.split(',').map((s) => s.trim().toUpperCase()).filter(Boolean)
  return {
    mode: 'aggregate', host: values.host ?? null, project: values.project,
    ref: values.ref ?? null, source: values.source ?? null, last, statuses, ...common,
  }
}
```

- [ ] **Step 5: Запустить тест**

Run: `node --test test/cli-args.test.mjs`
Expected: `# pass 7`, `# fail 0`.

- [ ] **Step 6: Commit**

```bash
git add package.json .gitignore src/cli-args.mjs test/cli-args.test.mjs
git commit -m "feat: каркас пакета и разбор аргументов CLI"
```

---

### Task 2: Модель — сырой пайплайн в дерево спанов

**Files:**
- Create: `src/model.mjs`, `test/fixtures.mjs`
- Test: `test/model.test.mjs`

**Interfaces:**
- Consumes: формат сырого пайплайна (раздел «Формат данных»).
- Produces:
  - `buildTree(raw, { baseUrl: string, origin?: number, now?: number })` → спан `pipeline`. `origin` по умолчанию `Date.parse(raw.pipeline.createdAt)`, `now` — `Date.now()`.
  - `shardGroupName(name: string)` → имя без суффикса шарда.
  - `test/fixtures.mjs`: `T0`, `at(sec)`, `job(name, stage, opts)`, `rawPipeline(jobs, opts)` — используются и в задаче 4.

- [ ] **Step 1: Создать фабрики фикстур**

`test/fixtures.mjs`:

```js
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
  finishedAt = null, project = 'g/p', downstream = {},
} = {}) {
  return {
    project,
    pipeline: { id, iid, status, createdAt, finishedAt, ref: 'master', path: `/${project}/-/pipelines/${iid}` },
    jobs: [...jobs].reverse(),
    downstream,
  }
}
```

- [ ] **Step 2: Написать падающий тест**

`test/model.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildTree, shardGroupName } from '../src/model.mjs'
import { T0, at, job, rawPipeline } from './fixtures.mjs'

const baseUrl = 'https://h'
const find = (span, name) => span.name === name ? span : span.children.map((c) => find(c, name)).find(Boolean)

test('стейджи идут в порядке запуска, границы стейджа — по его джобам', () => {
  const tree = buildTree(rawPipeline([
    job('prepare:checksum', 'prepare', { start: 0, end: 60 }),
    job('cache:npm', 'cache', { start: 5, end: 120 }),
    job('lint', 'build', { start: 125, end: 200, deps: ['cache:npm'] }),
    job('build:server', 'build', { start: 130, end: 300, deps: ['cache:npm'], queued: 2 }),
    job('deploy', 'deploy', { status: 'MANUAL' }),
  ], { status: 'MANUAL' }), { baseUrl })

  assert.equal(tree.kind, 'pipeline')
  assert.equal(tree.name, '#1')
  assert.equal(tree.start, 0)
  assert.equal(tree.end, 300_000)
  assert.equal(tree.url, 'https://h/g/p/-/pipelines/1')
  assert.deepEqual(tree.children.map((s) => s.name), ['prepare', 'cache', 'build', 'deploy'])

  const build = find(tree, 'build')
  assert.equal(build.kind, 'stage')
  assert.equal(build.start, 125_000)
  assert.equal(build.end, 300_000)
  assert.deepEqual(build.children.map((s) => s.name), ['lint', 'build:server'])

  const server = find(tree, 'build:server')
  assert.equal(server.queued, 2000)
  assert.equal(server.stage, 'build')
  assert.equal(server.status, 'success')
  assert.deepEqual(server.deps, ['gid://gitlab/Ci::Build/cache:npm'])
})

test('manual-джоба без старта остаётся в дереве без времени и не влияет на стейдж', () => {
  const tree = buildTree(rawPipeline([
    job('a', 'build', { start: 0, end: 10 }),
    job('deploy', 'build', { status: 'MANUAL' }),
  ]), { baseUrl })
  const deploy = find(tree, 'deploy')
  assert.equal(deploy.start, null)
  assert.equal(deploy.end, null)
  assert.equal(find(tree, 'build').end, 10_000)
})

test('ретраи уходят в attempts, deps указывают на актуальную попытку', () => {
  const tree = buildTree(rawPipeline([
    job('e2e', 'test', { start: 10, end: 20, status: 'FAILED', retried: true }),
    job('e2e', 'test', { start: 30, end: 40 }),
    job('report', 'reports', { start: 41, end: 45, deps: ['e2e', 'missing'] }),
  ]), { baseUrl })
  const e2e = find(tree, 'e2e')
  assert.equal(e2e.start, 30_000)
  assert.deepEqual(e2e.attempts, [
    { start: 10_000, end: 20_000, status: 'failed', url: 'https://h/g/p/-/jobs/e2e' },
  ])
  assert.deepEqual(find(tree, 'report').deps, ['gid://gitlab/Ci::Build/e2e'])
})

test('шарды одного стейджа объединяются в group, одиночная джоба — нет', () => {
  const tree = buildTree(rawPipeline([
    job('e2e: [1]', 'test', { start: 0, end: 50 }),
    job('e2e: [2]', 'test', { start: 5, end: 80 }),
    job('solo', 'test', { start: 1, end: 2 }),
  ]), { baseUrl })
  const stage = find(tree, 'test')
  assert.deepEqual(stage.children.map((s) => [s.kind, s.name]), [['group', 'e2e'], ['job', 'solo']])
  const group = stage.children[0]
  assert.equal(group.start, 0)
  assert.equal(group.end, 80_000)
  assert.deepEqual(group.children.map((s) => s.name), ['e2e: [1]', 'e2e: [2]'])
})

test('shardGroupName снимает оба формата суффикса', () => {
  assert.equal(shardGroupName('tests:e2e:master-stage: [9]'), 'tests:e2e:master-stage')
  assert.equal(shardGroupName('rspec 3/10'), 'rspec')
  assert.equal(shardGroupName('build:server'), 'build:server')
})

test('downstream: вложен в bridge, время отсчитывается от корня, bridge заканчивается вместе с ним', () => {
  const child = rawPipeline([job('inner', 'test', { start: 110, end: 200 })], {
    id: 'gid://gitlab/Ci::Pipeline/2', iid: '7', createdAt: at(100), project: 'other/proj',
  })
  const bridge = job('trigger', 'deploy', {
    kind: 'BRIDGE', start: 100, end: 150,
    downstream: { id: 'gid://gitlab/Ci::Pipeline/2', project: { fullPath: 'other/proj' } },
  })
  const tree = buildTree(rawPipeline([bridge], { downstream: { [bridge.id]: child } }), { baseUrl })
  const span = find(tree, 'trigger')
  assert.equal(span.kind, 'bridge')
  assert.equal(span.end, 200_000)
  const ds = span.children[0]
  assert.equal(ds.kind, 'pipeline')
  assert.equal(ds.start, 100_000)
  assert.equal(ds.url, 'https://h/other/proj/-/pipelines/7')
  assert.equal(find(ds, 'inner').start, 110_000)
  assert.equal(tree.end, 200_000)
})

test('идущая джоба заканчивается в now', () => {
  const tree = buildTree(rawPipeline([job('a', 'build', { start: 10, status: 'RUNNING' })]), {
    baseUrl, now: Date.parse(T0) + 50_000,
  })
  assert.equal(find(tree, 'a').end, 50_000)
})
```

- [ ] **Step 3: Запустить тест и убедиться, что он падает**

Run: `node --test test/model.test.mjs`
Expected: FAIL, `Cannot find module '…/src/model.mjs'`.

- [ ] **Step 4: Реализовать `src/model.mjs`**

```js
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
  const endOf = (j) => (j.finishedAt != null ? at(j.finishedAt) : j.startedAt != null ? now - origin : null)
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
```

- [ ] **Step 5: Запустить тест**

Run: `node --test test/model.test.mjs`
Expected: `# pass 7`, `# fail 0`.

- [ ] **Step 6: Commit**

```bash
git add src/model.mjs test/fixtures.mjs test/model.test.mjs
git commit -m "feat: модель спанов из сырого пайплайна"
```

---

### Task 3: Критический путь

**Files:**
- Create: `src/critical-path.mjs`
- Test: `test/critical-path.test.mjs`

**Interfaces:**
- Consumes: спан из `buildTree` (поля `id`, `kind`, `start`, `end`, `deps`, `children`).
- Produces: `criticalPath(root, scopeId = root.id)` → `{ ids: string[] /* хронологически */, gaps: [{ from, to, ms }] }`. `scopeId` — id пайплайна, стейджа, группы или bridge. Файл не содержит `import`: `render.mjs` встраивает его в HTML, убирая `export `.

- [ ] **Step 1: Написать падающий тест**

`test/critical-path.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { criticalPath } from '../src/critical-path.mjs'

const j = (id, start, end, deps = [], kind = 'job', children = []) =>
  ({ id, kind, name: id, start, end, deps, children, attempts: [], queued: null })
const node = (id, kind, children) => ({ id, kind, name: id, start: null, end: null, deps: [], children })
const pipe = (id, stages) => node(id, 'pipeline', stages)

test('цепочка: путь в хронологическом порядке, зазоры только положительные', () => {
  const root = pipe('p', [node('s', 'stage', [j('a', 0, 10), j('b', 15, 20, ['a']), j('c', 20, 30, ['b'])])])
  assert.deepEqual(criticalPath(root), {
    ids: ['a', 'b', 'c'],
    gaps: [{ from: 'a', to: 'b', ms: 5 }],
  })
})

test('ромб: выбирается зависимость, закончившаяся позже', () => {
  const root = pipe('p', [
    node('s1', 'stage', [j('a', 0, 10)]),
    node('s2', 'stage', [j('b', 10, 20, ['a']), j('c', 10, 40, ['a'])]),
    node('s3', 'stage', [j('d', 40, 50, ['b', 'c'])]),
  ])
  assert.deepEqual(criticalPath(root).ids, ['a', 'c', 'd'])
})

test('не запускавшиеся зависимости пропускаются, при равном end берётся первая', () => {
  const root = pipe('p', [node('s', 'stage', [
    j('x', null, null), j('b', 0, 10), j('c', 0, 10), j('d', 10, 20, ['x', 'b', 'c']),
  ])])
  assert.deepEqual(criticalPath(root).ids, ['b', 'd'])
})

test('scope стейджа: путь до его последней джобы', () => {
  const root = pipe('p', [
    node('build', 'stage', [j('a', 0, 10), j('b', 10, 30, ['a'])]),
    node('deploy', 'stage', [j('c', 30, 90, ['b'])]),
  ])
  assert.deepEqual(criticalPath(root, 'build').ids, ['a', 'b'])
})

test('bridge: путь заходит в downstream и возвращается к зависимостям bridge', () => {
  const inner = pipe('p2', [node('t', 'stage', [j('i1', 60, 70), j('i2', 75, 90, ['i1'])])])
  const root = pipe('p', [
    node('build', 'stage', [j('a', 0, 50), j('noise', 0, 5)]),
    node('deploy', 'stage', [j('br', 55, 90, ['a', 'noise'], 'bridge', [inner])]),
  ])
  assert.deepEqual(criticalPath(root), {
    ids: ['a', 'i1', 'i2', 'br'],
    gaps: [{ from: 'a', to: 'i1', ms: 10 }, { from: 'i1', to: 'i2', ms: 5 }],
  })
})

test('scope без запускавшихся джоб даёт пустой путь', () => {
  const root = pipe('p', [node('s', 'stage', [j('m', null, null)])])
  assert.deepEqual(criticalPath(root, 's'), { ids: [], gaps: [] })
})
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

Run: `node --test test/critical-path.test.mjs`
Expected: FAIL, `Cannot find module '…/src/critical-path.mjs'`.

- [ ] **Step 3: Реализовать `src/critical-path.mjs`**

```js
// Без import: render.mjs встраивает этот файл в HTML как обычный скрипт
export function criticalPath(root, scopeId = root.id) {
  const byId = new Map()
  const index = (s) => {
    byId.set(s.id, s)
    s.children.forEach(index)
  }
  index(root)

  // bridge — лист: его downstream обходится отдельно, через walk
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
```

- [ ] **Step 4: Запустить тест**

Run: `node --test test/critical-path.test.mjs`
Expected: `# pass 6`, `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add src/critical-path.mjs test/critical-path.test.mjs
git commit -m "feat: критический путь по previousStageJobsOrNeeds"
```

---

### Task 4: Агрегат по N пайплайнам

**Files:**
- Create: `src/aggregate.mjs`
- Test: `test/aggregate.test.mjs`

**Interfaces:**
- Consumes: `criticalPath(root)` из задачи 3; деревья из `buildTree` (задача 2); фикстуры `job`, `rawPipeline`.
- Produces:
  - `percentile(values: number[], p: number)` → `number | null`, метод nearest-rank.
  - `aggregate(trees: span[])` → агрегированный спан `pipeline` с `id: 'agg'` и `stats` (формат в разделе «Формат данных»). Дети сливаются по ключу `kind + name` (`'pipeline'` для пайплайна) и сортируются по p50 `start`, не запускавшиеся — в конце. Id детей — путь ключей: `agg/stage:build/job:lint`.

- [ ] **Step 1: Написать падающий тест**

`test/aggregate.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { aggregate, percentile } from '../src/aggregate.mjs'
import { buildTree } from '../src/model.mjs'
import { job, rawPipeline } from './fixtures.mjs'

const tree = (i, { buildEnd, lint }) => buildTree(rawPipeline([
  job('prepare', 'prepare', { start: 0, end: 10 }),
  job('build', 'build', { start: 10, end: buildEnd, deps: ['prepare'] }),
  ...(lint ? [job('lint', 'build', { start: 10, end: 20, deps: ['prepare'] })] : []),
], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) }), { baseUrl: 'https://h' })

const trees = [
  tree(1, { buildEnd: 100, lint: true }),
  tree(2, { buildEnd: 300, lint: true }),
  tree(3, { buildEnd: 200, lint: false }),
]
const child = (span, key) => span.children.find((c) => `${c.kind}:${c.name}` === key)

test('percentile: nearest-rank', () => {
  assert.equal(percentile([1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 50), 5)
  assert.equal(percentile([10, 1, 9, 2, 8, 3, 7, 4, 6, 5], 90), 9)
  assert.equal(percentile([7], 90), 7)
  assert.equal(percentile([], 50), null)
})

test('длина пайплайна и «до конца стейджа» — перцентиль по пайплайнам', () => {
  const agg = aggregate(trees)
  assert.equal(agg.id, 'agg')
  assert.equal(agg.name, '3 пайплайнов')
  assert.deepEqual(agg.stats.end, { p50: 200_000, p90: 300_000 })
  const build = child(agg, 'stage:build')
  assert.equal(build.id, 'agg/stage:build')
  assert.deepEqual(build.stats.end, { p50: 200_000, p90: 300_000 })
  assert.deepEqual(child(build, 'job:build').stats.duration, { p50: 190_000, p90: 290_000 })
})

test('присутствие и доля на критическом пути', () => {
  const build = child(aggregate(trees), 'stage:build')
  const lint = child(build, 'job:lint')
  assert.equal(lint.stats.present, 2)
  assert.equal(lint.stats.total, 3)
  assert.equal(lint.stats.critical, 0)
  assert.equal(child(build, 'job:build').stats.critical, 1)
  assert.deepEqual(lint.stats.samples.map((s) => s.tree), [0, 1])
})

test('полоска агрегата: start и end — p50, дети отсортированы по старту', () => {
  const agg = aggregate(trees)
  assert.deepEqual(agg.children.map((c) => c.name), ['prepare', 'build'])
  const build = child(agg, 'stage:build')
  assert.equal(build.start, 10_000)
  assert.equal(build.end, 200_000)
})

test('ретраи усредняются по запускам', () => {
  const withRetry = buildTree(rawPipeline([
    job('e2e', 'test', { start: 0, end: 5, retried: true, status: 'FAILED' }),
    job('e2e', 'test', { start: 6, end: 10 }),
  ]), { baseUrl: 'https://h' })
  const clean = buildTree(rawPipeline([job('e2e', 'test', { start: 0, end: 10 })]), { baseUrl: 'https://h' })
  const e2e = child(child(aggregate([withRetry, clean]), 'stage:test'), 'job:e2e')
  assert.equal(e2e.stats.retries, 0.5)
})
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

Run: `node --test test/aggregate.test.mjs`
Expected: FAIL, `Cannot find module '…/src/aggregate.mjs'`.

- [ ] **Step 3: Реализовать `src/aggregate.mjs`**

```js
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
```

- [ ] **Step 4: Запустить тест**

Run: `node --test test/aggregate.test.mjs`
Expected: `# pass 5`, `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add src/aggregate.mjs test/aggregate.test.mjs
git commit -m "feat: агрегат p50/p90 по нескольким пайплайнам"
```

---

### Task 5: Клиент GitLab

**Files:**
- Create: `src/gitlab.mjs`
- Test: `test/gitlab.test.mjs`

**Interfaces:**
- Produces:
  - `resolveHost(explicit: string | null, { exec? })` → `Promise<string>`.
  - `resolveToken(host, { env?, exec? })` → `Promise<string>`.
  - `createClient({ host, token, fetch? })` → `gql(query, variables) → Promise<data>` со свойством `gql.host`.
  - `fetchPipeline(gql, project, pipelineGid, depth = 0)` → `Promise<rawPipeline>`.
  - `listPipelines(gql, project, { ref, source, statuses, last })` → `Promise<{ ids: string[], counts: { [STATUS]: number } }>`.
  - `mrHeadPipeline(gql, project, mrIid)` → `Promise<string>` (gid).
  - `exec` — функция с сигнатурой `promisify(execFile)`: `(cmd, args) → Promise<{ stdout }>`.

Поведение GitLab проверено 2026-09-24 на `gitlab.litres.io` (19.2.6): неверный токен — HTTP 401; несуществующий проект — `data.project: null`; `pipelines(ref: null, source: null)` — без фильтра.

- [ ] **Step 1: Написать падающий тест**

`test/gitlab.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  createClient, fetchPipeline, listPipelines, mrHeadPipeline, resolveHost, resolveToken,
} from '../src/gitlab.mjs'

const ok = (data) => ({ ok: true, status: 200, json: async () => ({ data }) })
const fakeGql = (handler) => Object.assign(async (query, vars) => handler(query, vars), { host: 'h.example' })
const failingExec = async () => { throw new Error('glab: not found') }

test('createClient шлёт POST с Bearer-токеном и возвращает data', async () => {
  const calls = []
  const gql = createClient({ host: 'h.example', token: 'tok', fetch: async (url, init) => { calls.push([url, init]); return ok({ x: 1 }) } })
  assert.deepEqual(await gql('{ x }', { a: 1 }), { x: 1 })
  assert.equal(gql.host, 'h.example')
  const [url, init] = calls[0]
  assert.equal(url, 'https://h.example/api/graphql')
  assert.equal(init.method, 'POST')
  assert.equal(init.headers.Authorization, 'Bearer tok')
  assert.deepEqual(JSON.parse(init.body), { query: '{ x }', variables: { a: 1 } })
})

test('createClient: 401 и ошибки GraphQL превращаются в понятные ошибки', async () => {
  const unauthorized = createClient({ host: 'h.example', token: 't', fetch: async () => ({ ok: false, status: 401 }) })
  await assert.rejects(unauthorized('{ x }'), /h\.example.*401/)
  const broken = createClient({
    host: 'h.example', token: 't',
    fetch: async () => ({ ok: true, status: 200, json: async () => ({ errors: [{ message: 'Field x missing' }] }) }),
  })
  await assert.rejects(broken('{ x }'), /Field x missing/)
})

test('createClient держит не больше 4 запросов одновременно', async () => {
  let inFlight = 0
  let max = 0
  const gql = createClient({
    host: 'h', token: 't',
    fetch: async () => {
      inFlight++
      max = Math.max(max, inFlight)
      await new Promise((r) => setTimeout(r, 5))
      inFlight--
      return ok({})
    },
  })
  await Promise.all(Array.from({ length: 10 }, () => gql('{ x }')))
  assert.equal(max, 4)
})

const pipelineData = (id, jobs, pageInfo = { hasNextPage: false, endCursor: null }) => ({
  project: { pipeline: { id, iid: '1', status: 'SUCCESS', createdAt: 'T', finishedAt: null, ref: 'master', path: '/p', jobs: { nodes: jobs, pageInfo } } },
})

test('fetchPipeline склеивает страницы джоб', async () => {
  const gql = fakeGql((q, v) => v.after === 'c1'
    ? pipelineData(v.id, [{ id: 'j2', downstreamPipeline: null }])
    : pipelineData(v.id, [{ id: 'j1', downstreamPipeline: null }], { hasNextPage: true, endCursor: 'c1' }))
  const raw = await fetchPipeline(gql, 'g/p', 'P1')
  assert.equal(raw.project, 'g/p')
  assert.equal(raw.pipeline.id, 'P1')
  assert.equal(raw.pipeline.jobs, undefined)
  assert.deepEqual(raw.jobs.map((j) => j.id), ['j1', 'j2'])
  assert.deepEqual(raw.downstream, {})
})

test('fetchPipeline загружает downstream из его проекта', async () => {
  const seen = []
  const gql = fakeGql((q, v) => {
    seen.push([v.project, v.id])
    return v.id === 'P1'
      ? pipelineData('P1', [{ id: 'br', downstreamPipeline: { id: 'P2', project: { fullPath: 'other/proj' } } }])
      : pipelineData('P2', [])
  })
  const raw = await fetchPipeline(gql, 'g/p', 'P1')
  assert.deepEqual(seen, [['g/p', 'P1'], ['other/proj', 'P2']])
  assert.equal(raw.downstream.br.project, 'other/proj')
})

test('fetchPipeline: нет проекта или пайплайна', async () => {
  await assert.rejects(fetchPipeline(fakeGql(() => ({ project: null })), 'g/p', 'P1'), /Проект g\/p на h\.example/)
  await assert.rejects(fetchPipeline(fakeGql(() => ({ project: { pipeline: null } })), 'g/p', 'P1'), /Пайплайн P1/)
})

test('listPipelines фильтрует статусы, считает их и останавливается на last', async () => {
  const pages = {
    null: { nodes: [{ id: 'a', status: 'SUCCESS' }, { id: 'b', status: 'FAILED' }], pageInfo: { hasNextPage: true, endCursor: 'c1' } },
    c1: { nodes: [{ id: 'c', status: 'MANUAL' }, { id: 'd', status: 'SUCCESS' }], pageInfo: { hasNextPage: true, endCursor: 'c2' } },
  }
  const vars = []
  const gql = fakeGql((q, v) => { vars.push(v); return { project: { pipelines: pages[v.after] } } })
  const result = await listPipelines(gql, 'g/p', { ref: 'master', source: null, statuses: ['SUCCESS', 'MANUAL'], last: 2 })
  assert.deepEqual(result, { ids: ['a', 'c'], counts: { SUCCESS: 1, MANUAL: 1 } })
  assert.equal(vars.length, 2)
  assert.equal(vars[0].ref, 'master')
})

test('mrHeadPipeline возвращает gid и падает, если пайплайна нет', async () => {
  const gql = fakeGql(() => ({ project: { mergeRequest: { headPipeline: { id: 'P9' } } } }))
  assert.equal(await mrHeadPipeline(gql, 'g/p', '12'), 'P9')
  await assert.rejects(
    mrHeadPipeline(fakeGql(() => ({ project: { mergeRequest: { headPipeline: null } } })), 'g/p', '12'),
    /MR !12/,
  )
})

test('resolveToken: glab, затем GITLAB_TOKEN, затем ошибка', async () => {
  assert.equal(await resolveToken('h', { env: {}, exec: async () => ({ stdout: 'from-glab\n' }) }), 'from-glab')
  assert.equal(await resolveToken('h', { env: { GITLAB_TOKEN: 'from-env' }, exec: failingExec }), 'from-env')
  await assert.rejects(resolveToken('h', { env: {}, exec: failingExec }), /glab auth login --hostname h/)
})

test('resolveHost: явный хост, затем glab, затем ошибка', async () => {
  assert.equal(await resolveHost('x.example', { exec: failingExec }), 'x.example')
  assert.equal(await resolveHost(null, { exec: async () => ({ stdout: 'g.example\n' }) }), 'g.example')
  await assert.rejects(resolveHost(null, { exec: failingExec }), /--host/)
})
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

Run: `node --test test/gitlab.test.mjs`
Expected: FAIL, `Cannot find module '…/src/gitlab.mjs'`.

- [ ] **Step 3: Реализовать `src/gitlab.mjs`**

```js
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'

const run = promisify(execFile)
const MAX_PARALLEL = 4
const MAX_DOWNSTREAM_DEPTH = 3
const MAX_LIST_PAGES = 10

// glab может быть не установлен: ошибка значит «переходим к следующему источнику»
async function glabValue(exec, args) {
  try {
    const { stdout } = await exec('glab', args)
    return stdout.trim() || null
  } catch {
    return null
  }
}

export async function resolveHost(explicit, { exec = run } = {}) {
  const host = explicit ?? (await glabValue(exec, ['config', 'get', 'host']))
  if (!host) throw new Error('Не удалось определить хост GitLab: передай --host или ссылку на пайплайн')
  return host
}

export async function resolveToken(host, { env = process.env, exec = run } = {}) {
  const token = (await glabValue(exec, ['config', 'get', 'token', '--host', host])) ?? env.GITLAB_TOKEN
  if (!token) throw new Error(`Нет токена для ${host}. Выполни \`glab auth login --hostname ${host}\` или задай GITLAB_TOKEN`)
  return token
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
  do {
    const data = await gql(PIPELINE_QUERY, { project, id, after })
    if (!data.project) throw new Error(`Проект ${project} на ${gql.host} не найден или нет доступа`)
    if (!data.project.pipeline) throw new Error(`Пайплайн ${id} в проекте ${project} на ${gql.host} не найден`)
    const { jobs: page, ...rest } = data.project.pipeline
    pipeline = rest
    jobs.push(...page.nodes)
    after = page.pageInfo.hasNextPage ? page.pageInfo.endCursor : null
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
```

- [ ] **Step 4: Запустить тест**

Run: `node --test test/gitlab.test.mjs`
Expected: `# pass 10`, `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add src/gitlab.mjs test/gitlab.test.mjs
git commit -m "feat: GraphQL-клиент GitLab и загрузка пайплайнов"
```

---

### Task 6: HTML-отчёт

**Files:**
- Create: `src/render.mjs`, `src/template.html`
- Test: `test/render.test.mjs`

**Interfaces:**
- Consumes: исходник `src/critical-path.mjs`, деревья и агрегат из задач 2 и 4.
- Produces:
  - `render(report)` → `Promise<string>`, где `report = { meta, trees, agg }`:
    - `meta = { mode: 'single' | 'aggregate', host, project, label, statusCounts: { [STATUS]: n } | null, generatedAt: ISO }`;
    - `trees` — массив спанов `pipeline`;
    - `agg` — агрегированный спан или `null`.
  - `toScriptJson(value)` → JSON, безопасный внутри `<script>`.

В шаблоне два маркера: `/*__CRITICAL_PATH__*/` и `/*__DATA__*/null`.

Отличие от спецификации: в агрегате вместо переключателя p50/p90 подпись полоски показывает оба значения сразу (`p50 3m12s · p90 4m01s`). Переключатель ничего не добавляет, а кода требует.

- [ ] **Step 1: Написать падающий тест**

`test/render.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { render, toScriptJson } from '../src/render.mjs'

const report = {
  meta: { mode: 'single', host: 'h', project: 'g/p', label: '#1', statusCounts: null, generatedAt: '2026-09-24T00:00:00Z' },
  trees: [{ id: 'p', kind: 'pipeline', name: '</script><b>x', start: 0, end: 1, children: [], deps: [], attempts: [] }],
  agg: null,
}

test('toScriptJson экранирует закрывающий тег и разделители строк', () => {
  assert.equal(toScriptJson({ a: '</script>\u2028' }), '{"a":"\\u003c/script>\\u2028"}')
})

test('render подставляет данные и критический путь без export', async () => {
  const html = await render(report)
  assert.ok(!html.includes('/*__DATA__*/'))
  assert.ok(!html.includes('/*__CRITICAL_PATH__*/'))
  assert.ok(!html.includes('</script><b>'))
  assert.match(html, /\nfunction criticalPath\(root, scopeId = root\.id\)/)
  assert.ok(!/^export /m.test(html))
  const json = html.match(/const REPORT = (.*);\n/)[1]
  assert.deepEqual(JSON.parse(json), report)
})
```

- [ ] **Step 2: Запустить тест и убедиться, что он падает**

Run: `node --test test/render.test.mjs`
Expected: FAIL, `Cannot find module '…/src/render.mjs'`.

- [ ] **Step 3: Реализовать `src/render.mjs`**

```js
import { readFile } from 'node:fs/promises'

const TEMPLATE_URL = new URL('./template.html', import.meta.url)
const CRITICAL_PATH_URL = new URL('./critical-path.mjs', import.meta.url)

export function toScriptJson(value) {
  return JSON.stringify(value)
    .replace(/</g, '\\u003c')
    .replace(/\u2028/g, '\\u2028')
    .replace(/\u2029/g, '\\u2029')
}

export async function render(report) {
  const [template, criticalPathSource] = await Promise.all([
    readFile(TEMPLATE_URL, 'utf8'),
    readFile(CRITICAL_PATH_URL, 'utf8'),
  ])
  // функции-заменители: в данных могут встретиться последовательности вида $&
  return template
    .replace('/*__CRITICAL_PATH__*/', () => criticalPathSource.replace(/^export /gm, ''))
    .replace('/*__DATA__*/null', () => toScriptJson(report))
}
```

- [ ] **Step 4: Реализовать `src/template.html`**

```html
<!doctype html>
<html lang="ru">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>pipeline-trace</title>
<style>
  :root {
    --bg: #ffffff; --fg: #1f2328; --muted: #6e7781; --line: #e6e8eb; --row-alt: #f7f8fa; --hover: #eef1f6;
    --selected: #e3e8ff; --box: #f5c518; --ok: #6c5fc7; --fail: #e5484d; --warn: #f59e0b; --run: #3b82f6;
    --idle: #9aa4af; --queued: #cfd5dc; --critical: #111827;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #16171b; --fg: #e6e7ea; --muted: #9aa0a8; --line: #2a2d33; --row-alt: #1b1d22; --hover: #23262d;
      --selected: #2b3050; --box: #d4a90f; --ok: #8b7ff0; --fail: #f16a6e; --warn: #f5a524; --run: #5b9cf8;
      --idle: #6b7280; --queued: #3a3f47; --critical: #f9fafb;
    }
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--bg); color: var(--fg); font: 13px/1.4 -apple-system, system-ui, sans-serif; }
  header { position: sticky; top: 0; z-index: 2; background: var(--bg); border-bottom: 1px solid var(--line); }
  .top { display: flex; gap: 12px; align-items: baseline; flex-wrap: wrap; padding: 10px 12px 6px; }
  .top h1 { font-size: 15px; margin: 0; }
  .muted { color: var(--muted); }
  a { color: var(--ok); }
  button { font: inherit; color: inherit; background: none; border: 1px solid var(--line); border-radius: 4px; cursor: pointer; }
  .grid { display: grid; grid-template-columns: minmax(260px, 38%) 1fr; }
  .axis { position: relative; height: 20px; overflow: hidden; border-left: 1px solid var(--line); }
  .tick { position: absolute; top: 3px; font-size: 11px; color: var(--muted); padding-left: 3px; border-left: 1px solid var(--line); }
  .hint { padding: 3px 12px; font-size: 11px; color: var(--muted); }
  .row { display: grid; grid-template-columns: minmax(260px, 38%) 1fr; height: 22px; cursor: pointer; }
  .row:nth-child(even) { background: var(--row-alt); }
  .row:hover { background: var(--hover); }
  .row.selected { background: var(--selected); }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-right: 8px; line-height: 22px; }
  .tg { display: inline-block; min-width: 38px; margin-right: 4px; padding: 0 3px; font-size: 11px; line-height: 16px; }
  span.tg { border: 0; }
  .k { display: inline-block; width: 16px; text-align: center; font-size: 10px; font-weight: 700; border-radius: 3px; margin-right: 4px; background: var(--line); }
  .row.critical .name { font-weight: 600; }
  .lane { position: relative; overflow: hidden; border-left: 1px solid var(--line); }
  .bar { position: absolute; top: 5px; height: 12px; min-width: 2px; border-radius: 2px; }
  .box { background: var(--box); }
  .st-success { background: var(--ok); }
  .st-failed { background: var(--fail); }
  .st-warn { background: var(--warn); }
  .st-running, .st-pending { background: var(--run); }
  .st-canceled, .st-skipped, .st-manual, .st-created { background: var(--idle); }
  .queued { background: var(--queued); }
  .attempt { opacity: .3; background: var(--fail); }
  .whisker { top: 10px; height: 2px; background: var(--muted); }
  .gap { top: 10px; height: 0; border-top: 2px dashed var(--critical); }
  .row.critical .bar.main { outline: 2px solid var(--critical); outline-offset: 1px; }
  .label { position: absolute; top: 3px; font-size: 11px; white-space: nowrap; color: var(--muted); }
  #panel { position: fixed; top: 0; right: 0; bottom: 0; width: min(420px, 100vw); overflow: auto; z-index: 3;
           background: var(--bg); border-left: 1px solid var(--line); padding: 12px 16px; box-shadow: -4px 0 16px rgb(0 0 0 / .12); }
  #panel h2 { font-size: 15px; margin: 4px 28px 12px 0; word-break: break-all; }
  #panel table { border-collapse: collapse; width: 100%; }
  #panel th { text-align: left; font-weight: 400; color: var(--muted); padding: 3px 12px 3px 0; vertical-align: top; white-space: nowrap; }
  #panel td { padding: 3px 0; word-break: break-word; }
  #panel ol { padding-left: 20px; }
  #close { position: absolute; top: 10px; right: 12px; width: 26px; height: 26px; }
</style>
</head>
<body>
<header>
  <div class="top"><h1 id="title"></h1><span id="summary" class="muted"></span><button id="back" hidden>← к агрегату</button></div>
  <div class="grid"><div class="hint">Ctrl + колесо — масштаб, Shift + колесо — сдвиг, 0 — сброс, клик по стейджу — критический путь до его конца</div><div class="axis" id="axis"></div></div>
</header>
<main id="rows"></main>
<aside id="panel" hidden></aside>
<script>
/*__CRITICAL_PATH__*/
const REPORT = /*__DATA__*/null;
const KIND = { pipeline: 'P', stage: 'S', group: 'G', job: 'J', bridge: 'B' }
const STEPS = [1e3, 5e3, 1e4, 3e4, 6e4, 12e4, 3e5, 6e5, 9e5, 18e5, 36e5, 72e5]
const $ = (sel) => document.querySelector(sel)
const esc = (v) => String(v ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`)

function fmt(ms) {
  if (ms == null) return '—'
  if (ms < 1000) return `${Math.round(ms)}ms`
  const s = ms / 1000
  if (s < 60) return `${s.toFixed(1)}s`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m${String(Math.floor(s % 60)).padStart(2, '0')}s`
  return `${Math.floor(m / 60)}h${String(m % 60).padStart(2, '0')}m`
}

const state = { tree: null, index: new Map(), collapsed: new Set(), selected: null, scope: null, crit: { ids: new Set(), gaps: [] }, view: [0, 1] }
const isAgg = () => state.tree === REPORT.agg
const fullView = () => [0, (isAgg() ? state.tree.stats.end.p90 : state.tree.end) || 1]

function open(tree) {
  state.tree = tree
  state.index = new Map()
  state.collapsed = new Set()
  const walk = (s) => { state.index.set(s.id, s); if (s.kind === 'group') state.collapsed.add(s.id); s.children.forEach(walk) }
  walk(tree)
  state.selected = null
  state.scope = tree.id
  state.view = fullView()
  recompute()
  draw()
}

function recompute() {
  if (isAgg()) {
    const ids = [...state.index.values()].filter((s) => s.stats.critical >= 0.5).map((s) => s.id)
    state.crit = { ids: new Set(ids), gaps: [] }
  } else {
    const { ids, gaps } = criticalPath(state.tree, state.scope)
    state.crit = { ids: new Set(ids), gaps }
  }
}

const pos = (v) => ((v - state.view[0]) / (state.view[1] - state.view[0])) * 100
const bar = (cls, from, to) => `<div class="bar ${cls}" style="left:${pos(from)}%;width:${Math.max(pos(to) - pos(from), 0.1)}%"></div>`
const label = (from, to, text) => pos(to) > 80
  ? `<span class="label" style="right:calc(${100 - pos(from)}% + 4px)">${text}</span>`
  : `<span class="label" style="left:calc(${pos(to)}% + 4px)">${text}</span>`

function mainClass(s) {
  if (s.kind === 'pipeline' || s.kind === 'stage' || s.kind === 'group') return 'main box'
  if (isAgg()) return 'main st-success'
  return `main st-${s.status}${s.allowFailure && s.status === 'failed' ? ' st-warn' : ''}`
}

function lane(s) {
  if (isAgg()) {
    const st = s.stats
    if (!st.present) return '<span class="label" style="left:4px">не запускалась</span>'
    const parts = [`p50 ${fmt(st.duration.p50)} · p90 ${fmt(st.duration.p90)}`]
    if (st.present < st.total) parts.push(`${st.present}/${st.total}`)
    if (st.critical) parts.push(`крит. ${Math.round(st.critical * 100)}%`)
    return bar(mainClass(s), st.start.p50, st.end.p50) + bar('whisker', st.end.p50, st.end.p90) + label(st.start.p50, st.end.p90, parts.join(' · '))
  }
  if (s.start == null || s.end == null) return `<span class="label" style="left:4px">не запускалась · ${esc(s.status)}</span>`
  let html = ''
  for (const a of s.attempts) if (a.start != null && a.end != null) html += bar('attempt', a.start, a.end)
  if (s.queued) html += bar('queued', s.start - s.queued, s.start)
  for (const g of state.crit.gaps) if (g.to === s.id) html += bar('gap', s.start - g.ms, s.start)
  return html + bar(mainClass(s), s.start, s.end) + label(s.start, s.end, fmt(s.end - s.start))
}

function stageInfo(s) {
  if (isAgg()) return `до конца p50 ${fmt(s.stats.end.p50)} · p90 ${fmt(s.stats.end.p90)}`
  return s.end == null ? '' : `длина ${fmt(s.end - s.start)} · до конца ${fmt(s.end)}`
}

function rowHtml(s, depth) {
  const toggle = s.children.length
    ? `<button class="tg" data-toggle="${esc(s.id)}">${state.collapsed.has(s.id) ? '▸' : '▾'} ${s.children.length}</button>`
    : '<span class="tg"></span>'
  const info = s.kind === 'stage' ? ` <span class="muted">${stageInfo(s)}</span>` : ''
  const cls = `row${state.selected === s.id ? ' selected' : ''}${state.crit.ids.has(s.id) ? ' critical' : ''}`
  return `<div class="${cls}" data-id="${esc(s.id)}"><div class="name" style="padding-left:${depth * 14 + 6}px" title="${esc(s.name)}">`
    + `${toggle}<span class="k">${KIND[s.kind]}</span>${esc(s.name)}${info}</div><div class="lane">${lane(s)}</div></div>`
}

function drawAxis() {
  const [a, b] = state.view
  const step = STEPS.find((x) => (b - a) / x <= 10) ?? STEPS[STEPS.length - 1]
  let html = ''
  for (let t = Math.ceil(a / step) * step; t <= b; t += step) html += `<span class="tick" style="left:${pos(t)}%">${fmt(t)}</span>`
  $('#axis').innerHTML = html
}

function drawHeader() {
  const { meta } = REPORT
  const root = state.tree
  $('#title').textContent = `${meta.project} · ${isAgg() ? meta.label : root.name}`
  document.title = `pipeline-trace · ${$('#title').textContent}`
  if (isAgg()) {
    const counts = Object.entries(meta.statusCounts ?? {}).map(([k, v]) => `${v} ${k.toLowerCase()}`).join(', ')
    $('#summary').textContent = `агрегат: ${root.stats.total} пайплайнов (${counts}) · длина p50 ${fmt(root.stats.end.p50)} · p90 ${fmt(root.stats.end.p90)}`
  } else {
    $('#summary').innerHTML = `<a href="${esc(root.url)}" target="_blank" rel="noopener">${esc(root.name)}</a> · ${esc(root.status)} · ${esc(root.ref)} · длина ${fmt(root.end)}`
  }
  $('#back').hidden = !REPORT.agg || isAgg()
}

function draw() {
  drawHeader()
  drawAxis()
  const html = []
  const walk = (s, depth) => {
    html.push(rowHtml(s, depth))
    if (!state.collapsed.has(s.id)) s.children.forEach((c) => walk(c, depth + 1))
  }
  walk(state.tree, 0)
  $('#rows').innerHTML = html.join('')
  drawPanel()
}

const link = (url, text) => (url ? `<a href="${esc(url)}" target="_blank" rel="noopener">${esc(text)}</a>` : '—')
const p5090 = (x) => `p50 ${fmt(x.p50)} · p90 ${fmt(x.p90)}`

function details(s) {
  const rows = [['Тип', s.kind], ['Статус', esc(s.status) + (s.allowFailure ? ' (allow_failure)' : '')]]
  if (s.stage) rows.push(['Стейдж', esc(s.stage)])
  rows.push(['Старт', fmt(s.start)], ['Конец', fmt(s.end)], ['Длительность', s.start == null ? '—' : fmt(s.end - s.start)])
  if (s.queued != null) rows.push(['Очередь', fmt(s.queued)])
  if (s.deps.length) rows.push(['Зависит от', s.deps.map((id) => esc(state.index.get(id)?.name ?? id)).join('<br>')])
  if (s.attempts.length) {
    rows.push(['Попытки', s.attempts.map((a) => `${link(a.url, a.status)} ${a.start == null ? '' : fmt(a.end - a.start)}`).join('<br>')])
  }
  rows.push(['GitLab', link(s.url, 'открыть')])
  if (s.kind === 'stage' || s.kind === 'pipeline') rows.push(['Критический путь', `до конца «${esc(s.name)}» выделен в водопаде`])
  return rows
}

function aggDetails(s) {
  const st = s.stats
  const rows = [
    ['Тип', s.kind], ['Запускалась', `${st.present} из ${st.total}`],
    ['Старт', p5090(st.start)], ['Длительность', p5090(st.duration)], ['До конца', p5090(st.end)],
  ]
  if (st.queued.p50 != null) rows.push(['Очередь', p5090(st.queued)])
  if (s.kind === 'job' || s.kind === 'bridge') {
    rows.push(['Ретраев в среднем', st.retries.toFixed(2)], ['На критическом пути', `${Math.round(st.critical * 100)}% пайплайнов`])
  }
  const samples = [...st.samples].sort((a, b) => (b.end - b.start) - (a.end - a.start))
  rows.push(['Пайплайны', `<ol>${samples.map((x) => {
    const tree = REPORT.trees[x.tree]
    return `<li><a href="#" data-open-tree="${x.tree}">${esc(tree.name)}</a> ${fmt(x.end - x.start)} · ${link(tree.url, 'GitLab')}</li>`
  }).join('')}</ol>`])
  return rows
}

function drawPanel() {
  const s = state.selected && state.index.get(state.selected)
  const panel = $('#panel')
  panel.hidden = !s
  if (!s) return
  const rows = isAgg() ? aggDetails(s) : details(s)
  panel.innerHTML = `<button id="close" aria-label="Закрыть">×</button><h2>${esc(s.name)}</h2>`
    + `<table>${rows.map(([k, v]) => `<tr><th>${k}</th><td>${v}</td></tr>`).join('')}</table>`
}

document.addEventListener('click', (e) => {
  const toggle = e.target.closest('[data-toggle]')
  if (toggle) {
    const id = toggle.dataset.toggle
    state.collapsed.has(id) ? state.collapsed.delete(id) : state.collapsed.add(id)
    return draw()
  }
  const openTree = e.target.closest('[data-open-tree]')
  if (openTree) {
    e.preventDefault()
    return open(REPORT.trees[Number(openTree.dataset.openTree)])
  }
  if (e.target.closest('#close')) { state.selected = null; return draw() }
  if (e.target.closest('#back')) return open(REPORT.agg)
  const row = e.target.closest('.row')
  if (!row || e.target.closest('a')) return
  const s = state.index.get(row.dataset.id)
  state.selected = s.id
  if (!isAgg() && (s.kind === 'stage' || s.kind === 'pipeline')) { state.scope = s.id; recompute() }
  draw()
})

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') { state.selected = null; draw() }
  if (e.key === '0' && !e.ctrlKey && !e.metaKey) { state.view = fullView(); draw() }
})

$('#rows').addEventListener('wheel', (e) => {
  const laneEl = e.target.closest('.lane')
  if (!laneEl || !(e.ctrlKey || e.shiftKey)) return
  e.preventDefault()
  const rect = laneEl.getBoundingClientRect()
  const f = (e.clientX - rect.left) / rect.width
  const [a, b] = state.view
  const w = b - a
  if (e.ctrlKey) {
    const nw = Math.max(w * (e.deltaY > 0 ? 1.25 : 0.8), 1000)
    const left = a + f * w - f * nw
    state.view = [left, left + nw]
  } else {
    const d = ((e.deltaY || e.deltaX) / rect.width) * w
    state.view = [a + d, b + d]
  }
  draw()
}, { passive: false })

open(REPORT.agg ?? REPORT.trees[0])
</script>
</body>
</html>
```

- [ ] **Step 5: Запустить тест**

Run: `node --test test/render.test.mjs`
Expected: `# pass 2`, `# fail 0`.

- [ ] **Step 6: Commit**

```bash
git add src/render.mjs src/template.html test/render.test.mjs
git commit -m "feat: HTML-отчёт с водопадом и панелью деталей"
```

---

### Task 7: Бинарь, README и проверка на живых пайплайнах

**Files:**
- Create: `bin/pipeline-trace.mjs`, `README.md`

**Interfaces:**
- Consumes: всё из задач 1–6.
- Produces: исполняемый `pipeline-trace`. Код выхода `0` при успехе, `1` при ошибке. Путь к HTML печатается в stdout, прогресс — в stderr.

- [ ] **Step 1: Реализовать `bin/pipeline-trace.mjs`**

```js
#!/usr/bin/env node
import { execFile } from 'node:child_process'
import { writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { parseCliArgs } from '../src/cli-args.mjs'
import { createClient, fetchPipeline, listPipelines, mrHeadPipeline, resolveHost, resolveToken } from '../src/gitlab.mjs'
import { buildTree } from '../src/model.mjs'
import { aggregate } from '../src/aggregate.mjs'
import { render } from '../src/render.mjs'

const OPENER = { darwin: 'open', win32: 'explorer' }[process.platform] ?? 'xdg-open'
const slug = (s) => s.replace(/[^\w.-]+/g, '-')

async function main() {
  const args = parseCliArgs(process.argv.slice(2))
  if (args.mode === 'help') {
    console.log(args.usage)
    return
  }
  const host = await resolveHost(args.host)
  const gql = createClient({ host, token: await resolveToken(host) })

  let ids
  let statusCounts = null
  let suffix
  if (args.mode === 'pipeline') {
    ids = [`gid://gitlab/Ci::Pipeline/${args.pipelineId}`]
    suffix = args.pipelineId
  } else if (args.mode === 'mr') {
    ids = [await mrHeadPipeline(gql, args.project, args.mrIid)]
    suffix = `mr${args.mrIid}`
  } else {
    ;({ ids, counts: statusCounts } = await listPipelines(gql, args.project, args))
    if (ids.length === 0) throw new Error('Под фильтры не попал ни один пайплайн: проверь --ref, --source и --status')
    suffix = args.ref ?? args.source ?? 'all'
  }

  process.stderr.write(`Загружаю пайплайнов: ${ids.length}\n`)
  const raws = await Promise.all(ids.map((id) => fetchPipeline(gql, args.project, id)))
  const trees = raws.map((raw) => buildTree(raw, { baseUrl: `https://${host}` }))
  const isAggregate = args.mode === 'aggregate'
  const meta = {
    mode: isAggregate ? 'aggregate' : 'single',
    host,
    project: args.project,
    label: isAggregate ? [args.ref, args.source].filter(Boolean).join(' · ') || 'все пайплайны' : trees[0].name,
    statusCounts,
    generatedAt: new Date().toISOString(),
  }

  const out = resolve(args.out ?? `pipeline-trace-${slug(args.project)}-${slug(suffix)}.html`)
  await writeFile(out, await render({ meta, trees, agg: isAggregate ? aggregate(trees) : null }))
  console.log(out)
  // браузер не открылся — путь уже напечатан, это не ошибка
  if (args.open) execFile(OPENER, [out], () => {})
}

main().catch((err) => {
  console.error(`pipeline-trace: ${err.message}`)
  process.exitCode = 1
})
```

- [ ] **Step 2: Сделать бинарь исполняемым и проверить `--help`**

Run: `chmod +x bin/pipeline-trace.mjs && ./bin/pipeline-trace.mjs --help`
Expected: текст, начинающийся с `Использование:`.

- [ ] **Step 3: Написать `README.md`**

````markdown
# pipeline-trace

Показывает, на что уходит время GitLab-пайплайна: водопад стейджей и джоб в стиле трейсов Sentry, критический путь и p50/p90 по многим пайплайнам. Результат — один HTML-файл без внешних запросов.

## Установка

Нужен Node 22+.

```bash
git clone <url этого репозитория> && cd pipeline-trace && npm link
```

Токен берётся из `glab` (`glab auth login --hostname <host>`). Вместо этого можно задать `GITLAB_TOKEN` вместе с `GITLAB_HOST=<host>`: на другие хосты `GITLAB_TOKEN` не отправляется. Нужен доступ `read_api`.

## Запуск

```bash
# один пайплайн или пайплайн MR
pipeline-trace https://gitlab.example.com/group/project/-/pipelines/1025721
pipeline-trace https://gitlab.example.com/group/project/-/merge_requests/7632

# агрегат по последним 50 пайплайнам master
pipeline-trace --project group/project --ref master --last 50 --host gitlab.example.com

# агрегат по пайплайнам всех MR
pipeline-trace --project group/project --source merge_request_event --last 50 --host gitlab.example.com
```

Флаги: `--status success,manual|any` (по умолчанию `success,manual`), `--out файл.html`, `--no-open`.

Пайплайн, который остановился на ручной джобе, GitLab помечает `manual`. Если на ветке есть короткие служебные пайплайны в статусе `success` (например, Publish-коммиты), они смешаются с полными. Чтобы их отсечь, укажи `--status manual`.

## Как читать отчёт

- Строка стейджа: `длина` — от старта его первой джобы до конца последней; `до конца` — от создания пайплайна до конца стейджа.
- Жирные строки с обводкой — критический путь. Клик по стейджу пересчитывает путь до его конца. Пунктир — ожидание между джобами пути.
- Серый сегмент перед полоской — очередь раннера. Бледные полоски — прошлые попытки.
- В агрегате полоска идёт от p50 старта до p50 конца, линия после неё — до p90 конца. «крит. 80%» — джоба была на критическом пути в 80% пайплайнов.

## Разработка

```bash
npm test
```
````

- [ ] **Step 4: Прогнать все тесты**

Run: `npm test`
Expected: `# pass 37`, `# fail 0`.

- [ ] **Step 5: Проверить на живом пайплайне**

Возьми id полного пайплайна master в статусе `manual`:

```bash
glab api --hostname gitlab.litres.io "projects/platform%2Freact%2Fmonorepo/pipelines?ref=master&status=manual&per_page=1" | node -e 'let s = ""; process.stdin.on("data", (d) => (s += d)).on("end", () => console.log(JSON.parse(s)[0].id))'
```

Run (подставь id):

```bash
./bin/pipeline-trace.mjs https://gitlab.litres.io/platform/react/monorepo/-/pipelines/<id> --no-open --out /tmp/pt-single.html
```

Expected: в stdout — путь `/tmp/pt-single.html`, код выхода 0. Открой файл в браузере (Playwright MCP или вручную) и проверь:

- стейджи идут в порядке `prepare`, `cache`, `build`, …, у каждого есть `длина` и `до конца`;
- группа `tests:e2e:master-stage` свёрнута; после раскрытия у шардов с ретраями видны бледные полоски;
- `manual`-джобы (`release:production`) подписаны «не запускалась · manual»;
- клик по стейджу `build` меняет выделенный путь, клик по строке открывает панель, Esc её закрывает;
- в тёмной теме системы цвета читаются.

- [ ] **Step 6: Проверить агрегат и ошибки**

Run:

```bash
./bin/pipeline-trace.mjs --project platform/react/monorepo --host gitlab.litres.io --ref master --status manual --last 10 --no-open --out /tmp/pt-agg.html
./bin/pipeline-trace.mjs --project no/such --host gitlab.litres.io --no-open; echo "exit=$?"
GITLAB_TOKEN=bad GITLAB_HOST=example.invalid ./bin/pipeline-trace.mjs --project g/p --host example.invalid --no-open; echo "exit=$?"
```

Expected:
- первая команда пишет `Загружаю пайплайнов: 10` и путь к файлу. В отчёте шапка показывает `агрегат: 10 пайплайнов (10 manual)`, у джоб — `p50 … · p90 …`; ссылка на пайплайн в панели открывает его водопад, кнопка «← к агрегату» возвращает назад;
- вторая печатает `pipeline-trace: Проект no/such на gitlab.litres.io не найден или нет доступа` и `exit=1`;
- третья печатает строку, начинающуюся с `pipeline-trace:`, с сетевой ошибкой (`fetch failed`) и `exit=1`.

- [ ] **Step 7: Commit**

```bash
git add bin/pipeline-trace.mjs README.md
git commit -m "feat: бинарь pipeline-trace и README"
```
