# Стабильность, превышение и редизайн C: план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Отчёт сразу показывает, что оптимизировать: потери на ретраях, превышение стейджа узким местом, стабильность джоб и список «Куда направить силы»; интерфейс переходит на стиль C.

**Architecture:** Новый модуль `src/insights.mjs` (чистые функции) считает метрики по дереву одного пайплайна и по агрегату. `src/aggregate.mjs` добавляет в `stats` поля `retried` и `retryLoss`. `bin/pipeline-trace.mjs` кладёт результат в `report.insights`. `src/template.html` переписывается в стиле C и читает `REPORT.insights`.

**Tech Stack:** Node 22, ES-модули, `node:test`, vanilla JS/CSS в шаблоне. Зависимостей нет.

Спецификация: `docs/specs/2026-09-24-pipeline-trace-design.md`, раздел «Стабильность и превышение: куда направить силы».

## Global Constraints

- Без зависимостей; Node >= 22; ES-модули.
- `src/critical-path.mjs` не меняется и остаётся без `import`.
- `src/insights.mjs` импортирует только `./critical-path.mjs`; `src/aggregate.mjs` может импортировать `./insights.mjs`, обратного импорта нет.
- Порог пункта «Куда направить силы» — 10 000 мс, не больше 3 пунктов.
- Тексты для пользователя — на русском; все значения из GitLab в HTML — через `esc()`.
- Комментарии в коде — только там, где причина неочевидна из кода.

## Формат `report.insights`

```js
{
  trees: [ // по одному на каждое дерево из report.trees, тот же порядок
    { stages: { [stageId]: { id /* узкое место */, peersEnd, excess } },
      retryLoss: { [spanId]: ms },   // только ненулевые
      totalRetryLoss: ms,
      hotspots: [{ id, name, kind: 'retry' | 'excess', saving, retries?, stage?, excess? }],
      saving: ms },                  // сумма saving пунктов
  ],
  agg: null | { stages, retryLoss, totalRetryLoss, hotspots /* у 'retry' есть retried, present */, saving,
                stability: { [aggNodeId]: { retried, present } } },
  stability: null | { [stabilityKey]: { retried, present } },  // ключ — stabilityKey([...имена стейджей и bridge-предков, имя джобы])
}
```

---

### Task 1: `insights.mjs` для одного пайплайна

**Files:**
- Create: `src/insights.mjs`
- Create: `test/insights.test.mjs`

**Interfaces:**
- Consumes: `criticalPath(root)` из `src/critical-path.mjs`; деревья `buildTree` (`test/fixtures.mjs`: `job`, `rawPipeline`).
- Produces:
  - `export const MIN_SAVING_MS = 10_000`
  - `export function retryLoss(span): number`
  - `export function stageExcess(stage, endOf = (s) => s.end): { id, peersEnd, excess } | null`
  - `export function insights(tree): { stages, retryLoss, totalRetryLoss, hotspots, saving }` (формат выше).
  - Внутренние `walk`, `leaves`, `rank`, `sum` — задача 2 использует их в этом же файле.
  - `export function samplePipeline(i)` в `test/insights.test.mjs` — фикстура для задачи 2.

- [ ] **Step 1: Написать падающие тесты**

`test/insights.test.mjs`:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { buildTree } from '../src/model.mjs'
import { insights, retryLoss, stageExcess } from '../src/insights.mjs'
import { job, rawPipeline } from './fixtures.mjs'

export function samplePipeline(i = 1) {
  return rawPipeline([
    job('build:server', 'build', { start: 0, end: 302 }),
    job('build:static', 'build', { start: 10, end: 242 }),
    job('lint', 'build', { start: 0, end: 50, retried: true, status: 'FAILED' }),
    job('lint', 'build', { start: 60, end: 240 }),
    job('e2e: [1]', 'test', { start: 330, end: 780, deps: ['build:server'] }),
    job('e2e: [3]', 'test', { start: 331, end: 590, deps: ['build:server'], retried: true, status: 'FAILED' }),
    job('e2e: [3]', 'test', { start: 600, end: 850, deps: ['build:server'], retried: true, status: 'FAILED' }),
    job('e2e: [3]', 'test', { start: 860, end: 1318, deps: ['build:server'] }),
    job('report', 'report', { start: 1320, end: 1400, deps: ['e2e: [1]', 'e2e: [3]'] }),
  ], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) })
}
const tree = () => buildTree(samplePipeline(), { baseUrl: 'https://h' })
const find = (s, name) => (s.name === name ? s : s.children.map((c) => find(c, name)).find(Boolean))

test('retryLoss: от старта первой попытки до старта успешной', () => {
  const t = tree()
  assert.equal(retryLoss(find(t, 'e2e: [3]')), 529_000)
  assert.equal(retryLoss(find(t, 'lint')), 60_000)
  assert.equal(retryLoss(find(t, 'build:server')), 0)
})

test('stageExcess: узкое место и отрыв от предпоследней джобы', () => {
  const t = tree()
  assert.deepEqual(stageExcess(find(t, 'build')), { id: 'gid://gitlab/Ci::Build/build:server', peersEnd: 242_000, excess: 60_000 })
  assert.equal(stageExcess(find(t, 'report')), null)
})

test('insights: ретраи и превышение только с критического пути, без двойного счёта', () => {
  const t = tree()
  const r = insights(t)
  assert.deepEqual(r.hotspots.map((h) => [h.kind, h.name, h.saving]), [
    ['retry', 'e2e: [3]', 529_000],
    ['excess', 'build:server', 60_000],
  ])
  assert.equal(r.hotspots[0].retries, 2)
  assert.equal(r.saving, 589_000)
  assert.equal(r.totalRetryLoss, 589_000)
  assert.equal(Object.keys(r.retryLoss).length, 2)
  assert.equal(r.stages[find(t, 'test').id].excess, 538_000)
})
```

Пояснение: превышение стейджа `test` — 538 с, но 529 с из них уже учтены ретраями `e2e: [3]`; остаток 9 с меньше порога и в список не попадает. `lint` с ретраем не на критическом пути: его 60 с входят в `totalRetryLoss`, но не в `hotspots`.

- [ ] **Step 2: Запустить и убедиться, что тесты падают**

Run: `node --test test/insights.test.mjs`
Expected: FAIL, `Cannot find module '…/src/insights.mjs'`.

- [ ] **Step 3: Реализовать `src/insights.mjs`**

```js
import { criticalPath } from './critical-path.mjs'

export const MIN_SAVING_MS = 10_000
const MAX_HOTSPOTS = 3

const walk = (s, fn) => {
  fn(s)
  s.children.forEach((c) => walk(c, fn))
}
const leaves = (s) => (s.kind === 'job' || s.kind === 'bridge' ? [s] : s.children.flatMap(leaves))
const rank = (candidates) =>
  candidates.filter((c) => c.saving >= MIN_SAVING_MS).sort((a, b) => b.saving - a.saving).slice(0, MAX_HOTSPOTS)
const sum = (list) => list.reduce((n, c) => n + c.saving, 0)

// Столько пайплайн ждал из-за упавших попыток джобы
export function retryLoss(span) {
  const starts = span.attempts.map((a) => a.start).filter((v) => v != null)
  if (starts.length === 0 || span.start == null) return 0
  return Math.max(0, span.start - Math.min(...starts))
}

export function stageExcess(stage, endOf = (s) => s.end) {
  const ran = leaves(stage).filter((s) => endOf(s) != null).sort((a, b) => endOf(b) - endOf(a))
  if (ran.length < 2) return null
  const excess = endOf(ran[0]) - endOf(ran[1])
  return excess > 0 ? { id: ran[0].id, peersEnd: endOf(ran[1]), excess } : null
}

export function insights(tree) {
  const crit = new Set(criticalPath(tree).ids)
  const index = new Map()
  walk(tree, (s) => index.set(s.id, s))
  const stages = {}
  const losses = {}
  const candidates = []
  let totalRetryLoss = 0

  walk(tree, (s) => {
    if (s.kind === 'job' || s.kind === 'bridge') {
      const loss = retryLoss(s)
      if (loss === 0) return
      losses[s.id] = loss
      totalRetryLoss += loss
      if (crit.has(s.id)) candidates.push({ id: s.id, name: s.name, kind: 'retry', saving: loss, retries: s.attempts.length })
    } else if (s.kind === 'stage') {
      const ex = stageExcess(s)
      if (!ex) return
      stages[s.id] = ex
      if (!crit.has(ex.id)) return
      const bottleneck = index.get(ex.id)
      // ретраи узкого места уже стали отдельным пунктом — вычитаем, чтобы не считать дважды
      const saving = Math.max(0, ex.excess - retryLoss(bottleneck))
      candidates.push({ id: ex.id, name: bottleneck.name, kind: 'excess', saving, stage: s.name, excess: ex.excess })
    }
  })

  const hotspots = rank(candidates)
  return { stages, retryLoss: losses, totalRetryLoss, hotspots, saving: sum(hotspots) }
}
```

- [ ] **Step 4: Запустить тесты**

Run: `node --test test/insights.test.mjs`
Expected: `# pass 3`, `# fail 0`.

- [ ] **Step 5: Прогнать весь набор и закоммитить**

Run: `npm test` — `# fail 0`.

```bash
git add src/insights.mjs test/insights.test.mjs
git commit -m "feat: потери на ретраях, превышение стейджа и список «Куда направить силы»"
```

---

### Task 2: Метрики агрегата и подключение в бинарь

**Files:**
- Modify: `src/insights.mjs`
- Modify: `src/aggregate.mjs`
- Modify: `bin/pipeline-trace.mjs`
- Modify: `test/insights.test.mjs`

**Interfaces:**
- Consumes: `retryLoss`, `stageExcess`, внутренние `walk`/`rank`/`sum` из задачи 1; `aggregate` из `src/aggregate.mjs`; `samplePipeline` из `test/insights.test.mjs`.
- Produces:
  - в `stats` агрегированного узла: `retried: number` (число пайплайнов, где у узла были ретраи), `retryLoss: number` (средние потери по запускам, мс);
  - `export function aggInsights(agg)` — формат `report.insights.agg`;
  - `export function stabilityByName(agg): { [jobName]: { retried, present } }`;
  - `report.insights` в HTML-отчёте (формат в шапке плана).

- [ ] **Step 1: Написать падающие тесты**

В `test/insights.test.mjs` к импортам добавить `import { aggregate } from '../src/aggregate.mjs'` и `aggInsights, stabilityByName` в импорт из `../src/insights.mjs`. В конец файла:

```js
const cleanPipeline = (i) => rawPipeline([
  job('build:server', 'build', { start: 0, end: 302 }),
  job('build:static', 'build', { start: 10, end: 242 }),
  job('lint', 'build', { start: 60, end: 240 }),
  job('e2e: [1]', 'test', { start: 330, end: 780, deps: ['build:server'] }),
  job('e2e: [3]', 'test', { start: 860, end: 1318, deps: ['build:server'] }),
  job('report', 'report', { start: 1320, end: 1400, deps: ['e2e: [1]', 'e2e: [3]'] }),
], { id: `gid://gitlab/Ci::Pipeline/${i}`, iid: String(i) })

const aggOf = () => aggregate([
  buildTree(samplePipeline(1), { baseUrl: 'https://h' }),
  buildTree(cleanPipeline(2), { baseUrl: 'https://h' }),
])

test('агрегат: retried и средние retryLoss в stats', () => {
  const agg = aggOf()
  const shard = find(agg, 'e2e: [3]')
  assert.equal(shard.stats.retried, 1)
  assert.equal(shard.stats.present, 2)
  assert.equal(shard.stats.retryLoss, 264_500)
  assert.equal(find(agg, 'build:server').stats.retried, 0)
})

test('aggInsights: экономия взвешена долей на критическом пути, стабильность по узлам', () => {
  const agg = aggOf()
  const r = aggInsights(agg)
  assert.deepEqual(r.hotspots.map((h) => [h.kind, h.name, h.saving]), [
    ['excess', 'e2e: [3]', 273_500],
    ['retry', 'e2e: [3]', 264_500],
    ['excess', 'build:server', 60_000],
  ])
  assert.deepEqual(r.stability[find(agg, 'e2e: [3]').id], { retried: 1, present: 2 })
  assert.equal(r.totalRetryLoss, 264_500 + 30_000)
})

test('stabilityByName: метки для пайплайна, открытого из агрегата', () => {
  const s = stabilityByName(aggOf())
  assert.deepEqual(s['e2e: [3]'], { retried: 1, present: 2 })
  assert.deepEqual(s['build:server'], { retried: 0, present: 2 })
})
```

Пояснение к ожиданиям:
- `e2e: [3]`: потери 529 с и 0 — среднее 264,5 с; на критическом пути в обоих пайплайнах (`critical = 1`).
- Превышение `test` по p50: конец `[3]` = p50 старта 860 с + p50 длительности 458 с = 1318 с; конец `[1]` = 780 с; превышение 538 с; экономия `max(0, 538 − 264,5) × 1` = 273,5 с.
- `build`: превышение 60 с, `build:server` без ретраев на критическом пути в обоих — 60 с.
- `lint`: 60 с и 0 — среднее 30 с, не на критическом пути, входит только в `totalRetryLoss`.

- [ ] **Step 2: Запустить и убедиться, что тесты падают**

Run: `node --test test/insights.test.mjs`
Expected: FAIL — `aggInsights` не экспортируется.

- [ ] **Step 3: Добавить поля в `stats` агрегата**

В `src/aggregate.mjs`:

1. Добавить импорт: `import { retryLoss } from './insights.mjs'`.
2. В объекте `stats` после строки `retries: …` добавить:

```js
      retried: ran.filter((e) => e.span.attempts.length > 0).length,
      retryLoss: ran.length ? ran.reduce((n, e) => n + retryLoss(e.span), 0) / ran.length : 0,
```

- [ ] **Step 4: Добавить `aggInsights` и `stabilityByName`**

В конец `src/insights.mjs`:

```js
// конец агрегированного узла совпадает с концом его сплошной полоски в отчёте
const aggEnd = (s) => (s.stats.present ? s.stats.start.p50 + s.stats.duration.p50 : null)

export function aggInsights(agg) {
  const index = new Map()
  walk(agg, (s) => index.set(s.id, s))
  const stages = {}
  const losses = {}
  const stability = {}
  const candidates = []
  let totalRetryLoss = 0

  walk(agg, (s) => {
    if (s.kind === 'job' || s.kind === 'bridge') {
      stability[s.id] = { retried: s.stats.retried, present: s.stats.present }
      const loss = s.stats.retryLoss
      if (loss === 0) return
      losses[s.id] = loss
      totalRetryLoss += loss
      if (s.stats.critical > 0) {
        candidates.push({ id: s.id, name: s.name, kind: 'retry', saving: loss * s.stats.critical, retried: s.stats.retried, present: s.stats.present })
      }
    } else if (s.kind === 'stage') {
      const ex = stageExcess(s, aggEnd)
      if (!ex) return
      stages[s.id] = ex
      const bottleneck = index.get(ex.id)
      if (bottleneck.stats.critical === 0) return
      const saving = Math.max(0, ex.excess - bottleneck.stats.retryLoss) * bottleneck.stats.critical
      candidates.push({ id: ex.id, name: bottleneck.name, kind: 'excess', saving, stage: s.name, excess: ex.excess })
    }
  })

  const hotspots = rank(candidates)
  return { stages, retryLoss: losses, totalRetryLoss, hotspots, saving: sum(hotspots), stability }
}

export function stabilityByName(agg) {
  const out = {}
  walk(agg, (s) => {
    if (s.kind === 'job' || s.kind === 'bridge') out[s.name] = { retried: s.stats.retried, present: s.stats.present }
  })
  return out
}
```

- [ ] **Step 5: Подключить в бинарь**

В `bin/pipeline-trace.mjs`:

1. Добавить импорт: `import { aggInsights, insights, stabilityByName } from '../src/insights.mjs'`.
2. Строку `await writeFile(out, await render({ meta, trees, agg: isAggregate ? aggregate(trees) : null }))` заменить на:

```js
  const agg = isAggregate ? aggregate(trees) : null
  const report = {
    meta, trees, agg,
    insights: { trees: trees.map(insights), agg: agg && aggInsights(agg), stability: agg && stabilityByName(agg) },
  }
  await writeFile(out, await render(report))
```

- [ ] **Step 6: Тесты**

Run: `node --test test/insights.test.mjs` — `# pass 6`; `npm test` — `# fail 0`; `./bin/pipeline-trace.mjs --help` — печатает `Использование:`.

- [ ] **Step 7: Commit**

```bash
git add src/insights.mjs src/aggregate.mjs bin/pipeline-trace.mjs test/insights.test.mjs
git commit -m "feat: стабильность и превышение в агрегате, insights в отчёте"
```

---

### Task 3: Редизайн шаблона в стиле C

**Files:**
- Modify: `src/template.html`
- Modify: `README.md` (раздел «Как читать отчёт»)

**Interfaces:**
- Consumes: `REPORT.insights` (формат в шапке плана); всё существующее в шаблоне (спаны, `deps`, `after`, критический путь, связи, линии, панель, перетаскивание, `clampView`).
- Produces: ничего для других задач.

Эта задача — дизайн по описанию, а не транскрипция. Визуальный эталон — функции `main()` и `overX` в `/private/tmp/claude-501/-Users-vitalijmoskalevic-orca-workspaces-monorepo-pipeline-duration/6a75c64a-6da7-4b33-9077-b342902f1c21/scratchpad/mock-gen-2.mjs` (CSS-классы `.C …`, одобренный пользователем вариант «C + X»). Перенести стиль, а не разметку целиком: шаблон строит строки из данных.

Требования (все обязательны):

1. **Токены.** Все цвета — CSS-переменные на `:root` со значениями для светлой темы и переопределением под `@media (prefers-color-scheme: dark)`: фон страницы, фон карточки, линия, текст, приглушённый текст, успешная полоска, упавшая, `allow_failure`, критический путь (оранжевый), штриховка превышения, штриховка ретраев, ожидание, выделение строки, `--dep-up`, `--dep-down` (значения из текущего шаблона сохранить), метки стабильности (зелёная, жёлтая, красная). `:root { color-scheme: light dark }`, два `<meta name="theme-color">` с цветом фона для каждой темы (`media="(prefers-color-scheme: …)"`). `font-variant-numeric: tabular-nums` на `body`.
2. **Шапка.** Название (`проект · #iid` или метка агрегата) и чипы: «<длина> длина», «ретраи <totalRetryLoss>» (красный чип, если > 0), «можно сэкономить до <saving>» (оранжевый чип, если > 0), «<N> джобы». В агрегате длина — `p50 · p90`. Кнопка «← к агрегату» как сейчас. Строку-подсказку заменить кнопкой `?` с `aria-label="Управление"`, по клику показывающей список клавиш и жестов.
3. **Карточка «Куда направить силы»** — первой, если `hotspots` непуст. Пункт: экономия `−<saving>` оранжевым, имя джобы (`translate="no"`), причина:
   - `retry`, один пайплайн: «<retries> упавших попыток»; если есть `REPORT.insights.stability` — плюс «, ретраи в <retried> из <present> последних»;
   - `retry`, агрегат: «ретраи в <retried> из <present> пайплайнов»;
   - `excess`: «стейдж <stage> ждёт её <excess> после остальных»; в одном пайплайне плюс «, на критическом пути».
   Клик по пункту выбирает строку джобы (как клик по строке: выделение, панель, связи) и прокручивает к ней; свёрнутую группу, содержащую джобу, раскрыть. Пункт — `<button>`.
4. **Стейджи — карточки.** Строка стейджа становится заголовком карточки: имя полужирным; справа — «держит <имя узкого места> +<excess>» (если есть `stages[stageId]`), длина и «до конца» (в агрегате — p50 · p90). Джобы стейджа — строки внутри карточки. Карточки не ломают дерево: сворачивание стейджа, выделение, линии связей, критический путь, `after`-подпись `← имя` работают как раньше. Пайплайн (корневая строка) и downstream-пайплайны можно оставить обычными строками.
5. **Полоски.** Скруглённые, высота 10 px. Цвет: критический путь — оранжевый (вместо чёрной обводки), упавшая — красный, `allow_failure` — янтарный, прочие успешные — индиго, не запускавшиеся — подпись «не запускалась · <status>». Стейджи внутри карточек полоской не рисуются (метрики в заголовке); группа шардов — тонкая полоска на своём диапазоне.
6. **Превышение (вариант X).** Если у стейджа есть `stages[stageId]`: в строках этого стейджа — пунктирная вертикаль на `peersEnd`; у узкого места часть полоски правее `peersEnd` заштрихована (штриховка критического цвета) и после полоски подпись «<длительность> +<excess>» (превышение оранжевым, полужирным). В агрегате — по `REPORT.insights.agg.stages`.
7. **Ретраи.** В одном пайплайне упавшие попытки (`attempts`) рисуются в той же строке красной штриховкой, между концом попытки и стартом следующей — серая черта ожидания; подпись после полоски: «<длительность> · ↻<attempts.length> −<retryLoss>» (часть «↻… −…» красным). В агрегате — подпись «↻ <retried>/<present>» вместо попыток.
8. **Стабильность.** Справа в строке джобы — метка: зелёная «стабильна» (retried = 0), жёлтая «↻ k из N» (k/N < 0,3), красная (k/N ≥ 0,3). Источник: в агрегате — `insights.agg.stability[id]`; в пайплайне, открытом из агрегата, — `insights.stability[stabilityKey(путь)]`, где путь строится по state.parent так же: имена предков kind stage/bridge и имя джобы; без агрегата меток нет.
9. **Подписи у полосок** — одно число: длительность (в агрегате — p50). p90, присутствие и «крит. %» — в панели деталей. Присутствие < 100% — полоска полупрозрачная (opacity 0.5).
10. **Убрать:** буквы-бейджи P/S/J/G/B, рамки у кнопок сворачивания (шеврон ▸/▾ без рамки, число детей приглушённо), толстые жёлтые полоски стейджей, чёрную обводку критического пути.
11. **Панель деталей.** Сверху 2–3 крупных числа (длительность, старт, «до конца»; в агрегате p50 · p90), ниже таблица как сейчас; добавить строки «Потери на ретраях» и, если джоба — узкое место, «Превышение стейджа». `overscroll-behavior: contain`.
12. **Доступность.** Строки: `tabindex="0"`; ↑/↓ переводят фокус на соседнюю видимую строку, Enter выбирает (как клик), ←/→ сворачивают/разворачивают узел с детьми, Esc — как сейчас. Кнопка сворачивания: `aria-expanded`, `aria-label="Свернуть <имя>"`/«Развернуть <имя>». Видимый `:focus-visible` у строк и кнопок. `translate="no"` на именах джоб, стейджей и проекта. Сдвиг оси с клавиатуры: `[` и `]` сдвигают на 10% ширины окна (через `clampView`). После перерисовки фокус остаётся на той же строке.
13. **Сохранить без регрессий:** связи и линии (`drawLinks`, цвета выбранной джобы), `← parent`, подсветка `↑/↓`, критический путь и пересчёт по клику на стейдж, перетаскивание, Ctrl/Shift + колесо, клавиша `0`, `clampView`, `panel-open`, отступы дорожки 12/24 px, `esc()` на всех значениях из данных.

- [ ] **Step 1:** Реализовать требования 1–13 в `src/template.html`.
- [ ] **Step 2:** README, раздел «Как читать отчёт»: заменить пункты про подписи агрегата и критический путь на описание новых элементов (карточка «Куда направить силы», превышение `+…` со штриховкой, ретраи штриховкой и `↻`, метки стабильности, оранжевый критический путь, клавиши ↑/↓/Enter/←/→/`[`/`]`).
- [ ] **Step 3:** `npm test` — `# fail 0`. Синтаксическая проверка: отрендерить отчёт из `samplePipeline` (`test/insights.test.mjs`) через `render()` с `insights`, извлечь тело `<script>` и проверить `new Function(body)`.
- [ ] **Step 4: Commit**

```bash
git add src/template.html README.md
git commit -m "feat: редизайн C — стабильность, превышение, «Куда направить силы»"
```

---

### Task 4: Живая проверка

**Files:** без изменений кода; дефекты — отдельным fix-коммитом после согласования.

- [ ] **Step 1:** Собрать один пайплайн master со статусом `manual` и агрегат `--ref master --status manual --last 10` в scratchpad, открыть через `python3 -m http.server` и Playwright MCP.
- [ ] **Step 2:** Проверить и записать pass/fail: карточка «Куда направить силы» с пунктами и переходом к строке; чипы шапки; карточки стейджей с «держит … +…»; штриховка превышения и пунктирная вертикаль; ретраи штриховкой с подписью `↻N −…` (шарды `tests:e2e:master-stage`); метки стабильности в агрегате и в пайплайне, открытом из агрегата; линии связей и подсветка; клавиатура (Tab до строки, ↑/↓, Enter, ←/→, `[`/`]`, Esc) с видимым фокусом; тёмная тема; перетаскивание и панель.
- [ ] **Step 3:** Остановить сервер, проверить `git status` в обоих репозиториях, убрать лишние файлы в scratchpad.
