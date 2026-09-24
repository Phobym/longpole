# Выбор проекта в десктопе: план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** На стартовом окне — список проектов пользователя с поиском; экран проекта с веткой, последними пайплайнами и агрегатом; ручной ввод проекта не нужен.

**Architecture:** GraphQL-запросы списков — в `src/browse.mjs` (чистые функции над `gql`, тесты на подменённом клиенте). Главный процесс добавляет IPC `projects`, `branches`, `pipelines` через существующий `handle` (проверка отправителя). Форма переходит на два экрана: «Проекты» и «Проект». Отчёт строится существующим `build` в режимах `link` и `aggregate`.

**Tech Stack:** Node 22, Electron 44.4.5, `node:test`, vanilla HTML/CSS/JS.

Спецификация: `docs/specs/2026-09-24-desktop-design.md`, раздел «Выбор проекта».

## Global Constraints

- Корневой пакет без зависимостей; `src/browse.mjs` импортирует только встроенное (ничего из `desktop/`).
- Токен не уходит в окно формы; новые каналы — только через `handle(...)` в `desktop/app/main.mjs`.
- Тексты на русском; значения из GitLab в DOM — через `textContent`, не `innerHTML`.
- Комментарии — только где причина неочевидна.

---

### Task 1: `src/browse.mjs`

**Files:**
- Create: `src/browse.mjs`, `test/browse.test.mjs`

**Interfaces:**
- Produces:
  - `listProjects(gql, { search = '', after = null } = {}) → { items: [{ fullPath, name, lastActivityAt, defaultBranch }], next: string | null }`
  - `listBranches(gql, project, search = '') → string[]` — до 20 имён, основная ветка первой, если попала в выборку.
  - `recentPipelines(gql, project, { ref = null, after = null } = {}) → { items: [{ id, iid, status, source, createdAt, duration, commit: { sha, title } | null, author }], next }`; `id` — числовой id из gid, `status` в нижнем регистре, `duration` в мс или `null`.
  - Ошибка `Проект <project> на <gql.host> не найден или нет доступа`, если `data.project` пуст.

- [ ] **Step 1: Падающие тесты** — `test/browse.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { listBranches, listProjects, recentPipelines } from '../src/browse.mjs'

const fake = (handler) => Object.assign(async (q, v) => handler(q, v), { host: 'h.example' })

test('listProjects: поля, курсор, пустой поиск передаётся как null', async () => {
  const seen = []
  const gql = fake((q, v) => {
    seen.push(v)
    return { projects: { pageInfo: { hasNextPage: true, endCursor: 'c1' }, nodes: [
      { fullPath: 'g/p', nameWithNamespace: 'G / P', lastActivityAt: '2026-09-24T10:00:00Z', repository: { rootRef: 'main' } },
      { fullPath: 'g/empty', nameWithNamespace: 'G / Empty', lastActivityAt: '2026-09-23T10:00:00Z', repository: null },
    ] } }
  })
  const r = await listProjects(gql, { search: '  ' })
  assert.deepEqual(r, {
    items: [
      { fullPath: 'g/p', name: 'G / P', lastActivityAt: '2026-09-24T10:00:00Z', defaultBranch: 'main' },
      { fullPath: 'g/empty', name: 'G / Empty', lastActivityAt: '2026-09-23T10:00:00Z', defaultBranch: null },
    ],
    next: 'c1',
  })
  assert.equal(seen[0].search, null)
  await listProjects(gql, { search: 'mono', after: 'c1' })
  assert.deepEqual(seen[1], { search: 'mono', after: 'c1' })
})

test('listBranches: шаблон поиска и основная ветка первой', async () => {
  const seen = []
  const gql = fake((q, v) => {
    seen.push(v)
    return { project: { repository: { rootRef: 'master', branchNames: ['feature/a', 'master', 'fix/b'] } } }
  })
  assert.deepEqual(await listBranches(gql, 'g/p', 'a'), ['master', 'feature/a', 'fix/b'])
  assert.equal(seen[0].pattern, '*a*')
  await listBranches(gql, 'g/p')
  assert.equal(seen[1].pattern, '*')
})

test('recentPipelines: преобразование полей и курсор', async () => {
  const gql = fake(() => ({ project: { pipelines: { pageInfo: { hasNextPage: false, endCursor: null }, nodes: [
    { id: 'gid://gitlab/Ci::Pipeline/1026324', iid: '51256', status: 'MANUAL', source: 'push', createdAt: '2026-09-24T19:02:41+03:00', duration: 3952, commit: { shortId: '718e7e48', title: 'Publish' }, user: { username: 'u' } },
    { id: 'gid://gitlab/Ci::Pipeline/7', iid: '1', status: 'RUNNING', source: 'web', createdAt: '2026-09-24T19:00:00+03:00', duration: null, commit: null, user: null },
  ] } } }))
  assert.deepEqual(await recentPipelines(gql, 'g/p', { ref: 'master' }), {
    items: [
      { id: '1026324', iid: '51256', status: 'manual', source: 'push', createdAt: '2026-09-24T19:02:41+03:00', duration: 3_952_000, commit: { sha: '718e7e48', title: 'Publish' }, author: 'u' },
      { id: '7', iid: '1', status: 'running', source: 'web', createdAt: '2026-09-24T19:00:00+03:00', duration: null, commit: null, author: null },
    ],
    next: null,
  })
})

test('нет проекта — понятная ошибка', async () => {
  const gql = fake(() => ({ project: null }))
  await assert.rejects(listBranches(gql, 'no/such'), /Проект no\/such на h\.example/)
  await assert.rejects(recentPipelines(gql, 'no/such'), /Проект no\/such на h\.example/)
})
```

Run: `node --test test/browse.test.mjs` → FAIL (модуля нет).

- [ ] **Step 2: Реализация** — `src/browse.mjs`:

```js
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
```

- [ ] **Step 3:** `node --test test/browse.test.mjs` → `# pass 4`; `npm test` → `# fail 0`.
- [ ] **Step 4: Commit** — `git add src/browse.mjs test/browse.test.mjs && git commit -m "feat: списки проектов, веток и пайплайнов для выбора в приложении"`

---

### Task 2: Экраны «Проекты» и «Проект» в приложении

**Files:**
- Modify: `desktop/app/main.mjs`, `desktop/app/preload.cjs`, `desktop/app/ui/index.html`, `desktop/app/ui/form.js`, `desktop/app/ui/form.css`, `README.md`

**Interfaces:**
- Consumes: `listProjects`, `listBranches`, `recentPipelines` (`app/core/browse.mjs` после `sync-core`), существующие `handle`, `tokens`, `resolveToken`, `createClient`, `build`.
- Produces: мост `window.app` дополняется `projects({ host, search, after })`, `branches({ host, project, search })`, `pipelines({ host, project, ref, after })` → `{ ok, value?, error? }`.

Требования:

1. `main.mjs`: функция `clientFor(host)` — токен из хранилища, иначе `resolveToken`, иначе ошибка `Нет токена для <host>: добавь его в форме (поле «Токен»)` (та же, что уже в `build`; вынести общий код, не дублировать); каналы `projects`, `branches`, `pipelines` через `handle(...)`; хост проверяется `resolveHost`.
2. `preload.cjs`: три новые функции моста, ничего больше.
3. Экран «Проекты» (стартовый): выбор хоста и управление токеном — как сейчас; если для выбранного хоста нет токена — вместо списка подсказка и поле токена. Поле «Вставь ссылку на пайплайн или MR» с кнопкой «Построить» (режим `link`). Поиск проектов (`<input type="search">`, запрос через 300 мс после последнего ввода, устаревшие ответы отбрасываются по счётчику запроса), список проектов: имя с группой, путь (`translate="no"`), «активность <относительное время>» (`Intl.RelativeTimeFormat('ru', { numeric: 'auto' })`), основная ветка чипом; «Показать ещё» по `next`; пустой результат — «Ничего не найдено»; ошибка — блок `role="alert"`. Каждая строка — `<button>`. Ручная форма агрегата (поля проект/ref/source) удаляется из интерфейса.
4. Экран «Проект»: заголовок — имя проекта, кнопка «← Проекты» (и Esc) возвращает к списку с сохранённым поиском и прокруткой; ветка — `<input>` с `<datalist>` из `branches` (запрос через 300 мс), по умолчанию основная ветка; смена ветки перезагружает список пайплайнов. Список последних пайплайнов (`pipelines`): `#iid`, метка статуса (цвета как в отчёте), заголовок коммита (обрезка многоточием), автор, относительное время, длительность (`5m17s`), «Показать ещё»; клик по строке (`<button>`) строит трейс: `build({ mode: 'link', url: \`https://${host}/${project}/-/pipelines/${id}\` })`. Блок агрегата: чипы статусов (SUCCESS, MANUAL, FAILED, CANCELED, RUNNING, «любой»; по умолчанию SUCCESS + MANUAL), число N (по умолчанию 50, 1–500) и кнопка «Агрегат по N пайплайнам» → `build({ mode: 'aggregate', host, project, ref, source: '', last, statuses })`. Прогресс «Загружаю k из N…» и ошибки — у той кнопки или строки, что запустила сборку.
5. История слева — как сейчас: для `link` клик заполняет поле ссылки на стартовом экране; для `aggregate` открывает экран проекта с веткой и статусами из записи.
6. Доступность: все интерактивные элементы — кнопки и поля с подписями, `:focus-visible`, `aria-live="polite"` для статуса загрузки списков.
7. README (раздел «Десктопное приложение»): описать выбор проекта вместо ручного ввода.

- [ ] **Step 1:** Реализовать требования 1–7.
- [ ] **Step 2:** `npm test` и `cd desktop && npm test` → `# fail 0`; `cd desktop && npm run sync-core && PIPELINE_TRACE_SMOKE=1 npx electron .` → `smoke: ok`.
- [ ] **Step 3:** Проверка UI в Playwright на копии `desktop/app/ui` в scratchpad с заглушкой `window.app` (проекты с `next`, ветки, пайплайны, ошибка): поиск, «Показать ещё», переход на экран проекта и обратно, смена ветки, клик по пайплайну вызывает `build` с правильной ссылкой, кнопка агрегата — с правильными полями; светлая и тёмная тема.
- [ ] **Step 4:** Пересобрать macOS: `cd desktop && CSC_IDENTITY_AUTO_DISCOVERY=false npm run dist`; `codesign --verify --deep --strict dist/mac-arm64/pipeline-trace.app`; smoke из `dist/mac-arm64`.
- [ ] **Step 5: Commit** — `git add desktop/app README.md && git commit -m "feat(desktop): выбор проекта, экран проекта с ветками и пайплайнами"`
