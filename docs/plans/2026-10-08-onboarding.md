# Онбординг по событиям и маскот-жираф — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Подсказки с жирафом появляются у элемента в момент первой встречи с ним; жираф живёт в пустых состояниях, загрузке и inline-ошибках; подсказки выключаются и сбрасываются в настройках.

**Architecture:** Ядро хранит `tips` и `seenTips` в `settings.json`. Фронтенд получает `shared/ui/giraffe.tsx` и слайс `features/onboarding`: чистая функция выбора `pickTip`, zustand-стор с реестром, обёртка `<Tip id when>` вокруг элемента-якоря (Radix `Popover` в дереве компонента), `OnboardingSync` (настройки → стор) и блок настроек. Виджеты и страницы оборачивают свои элементы в `<Tip>`.

**Tech Stack:** Rust 2024 (`longpole-core`, `serde`, `ts-rs` 12), React 19, Zustand 5, TanStack Query, Radix (`radix-ui`), i18next / react-i18next, Tailwind v4, `node:test`.

Спека — `docs/specs/2026-10-08-onboarding-design.md` (далее «спека, § N»). Правила репозитория: `docs/agents/code-smells.md` (проход по диффу перед коммитом), комментарии и тексты — по-русски, имена тестов ядра — по-русски через `_`.

## Global Constraints

- Все команды npm и cargo — из `app/`.
- Сгенерированные ts-rs типы в `app/src/shared/api/schema/` коммитятся: после изменения Rust-типов с `#[ts(export)]` — `cargo test -p longpole-core`, затем `git add src/shared/api/schema`. CI падает на незакоммиченной схеме.
- Перед коммитом Rust — `cargo fmt --all`. Pre-commit (lefthook) гоняет `typecheck`, `lint:fsd`, `check:locales`, `cargo fmt --check`; хуки обязательны во всех задачах. В worktree без `npm ci` в `app/` хуки молча пропускаются — сначала `npm ci`.
- FSD (Steiger): `features` не импортируют `features` и `widgets`; `entities` не импортируют `features`. Поэтому `<Tip>` ставится в `widgets`, `pages` и `app`, никогда в `entities/*` и в чужих `features/*`.
- Импорты относительные, без алиасов; слайс — через его `index.ts`; `shared/ui` — по файлу (`../../../shared/ui/giraffe`).
- Ключи i18n: новый ключ — во все 8 файлов `app/src/shared/i18n/locales/{ru,en,fr,es,de,it,zh,ja}.json`, иначе `check:locales` падает; ключ, которого нет в `ru.json`, не проходит `tsc`.
- Сохранённый HTML-отчёт (`app/src/app/report/main.tsx`) рендерит `ReportPage` без `QueryClientProvider` и моста Tauri: код, который попадает в отчёт (`<Tip>`, `useOnboardingVisit`, `TipBubble`), не вызывает react-query и Tauri. Только zustand и i18n.
- Иконки — только lucide; анимации — только у подсказки и позы `loading`, обе гасятся `motion-reduce:animate-none`.
- Цвета маскота (спека, § 3): `mascot` `#e8701a` / `#ff8a3d`, `mascot-bg` `#fff4e8` / `#2e2216`, `mascot-border` `#f2c79c` / `#5a3d10` (светлая / тёмная).

## Отступления от спеки (вносятся в спеку в задаче 7)

1. `reportExpired` показывается системным окном (`report-route.tsx:34-39`), inline-ошибки там нет — жираф `oops` только под полем «Добавить проект» и в блоке токена.
2. `token.create` привязана ко всему блоку токена, а не к ссылке «Создать на {host}»: `TokenBlock` лежит в `features/manage-token`, а фича не может импортировать `features/onboarding`.
3. Жираф `loading` 72px — рядом с кнопкой агрегата и на экране загрузки отчёта (`form.reportView.loading`); в кнопке «Построить» шапки и в строке пайплайна остаётся текст прогресса — там нет места под 72px, а одиночная сборка короткая.
4. Esc: подсказка в `onEscapeKeyDown` вызывает `stopPropagation()` (слушатель Radix — capture на `document`), и до `useReportKeys` и корневого обработчика событие не доходит. Корневой обработчик не меняется.
5. Визит экрана отмечают только `ProjectPage` и `ReportPage`: у главного экрана и настроек нет `enter`-подсказок.
6. Подсказка уходит из `active` при размонтировании якоря с задержкой в микротаск: `StrictMode` в dev размонтирует и монтирует эффект сразу, и без задержки подсказка гасла бы в момент показа.

## Задачи и зависимости

| № | Задача | Зависит от |
|---|---|---|
| 1 | Ядро: `tips` и `seenTips` в настройках | — |
| 2 | Жираф: компонент, токены, пустые состояния, загрузка, ошибки | — |
| 3 | Онбординг: `pickTip` с тестом, реестр, стор | 1 |
| 4 | Онбординг: `<Tip>`, пузырь, синхронизация, блок настроек, словари | 2, 3 |
| 5 | Подсказки приложения: диалог, токен, проект, шапка | 4 |
| 6 | Подсказки отчёта | 4 |
| 7 | Документы и приёмка | 5, 6 |

Задачи 1 и 2 независимы. Задачи 5 и 6 независимы.

---

### Task 1: Ядро — `tips` и `seenTips` в настройках

**Files:**
- Modify: `app/core/src/settings.rs` (`Stored`, `AppSettings`, `SettingsPatch`, `get`, `update`)
- Modify: `app/core/tests/settings.rs` (литерал `AppSettings` в тесте `патч_меняет_только_переданное…`, два новых теста)
- Modify: `app/src/app/dev/mock.ts:12`
- Regenerate: `app/src/shared/api/schema/AppSettings.ts`, `app/src/shared/api/schema/SettingsPatch.ts`

**Interfaces:**
- Produces (Rust): `AppSettings { locale, theme, last_project, tips: bool, seen_tips: Vec<String> }`, `SettingsPatch { …, tips: Option<bool>, seen_tips: Option<Vec<String>> }`.
- Produces (TS): `AppSettings = { locale, theme, lastProject, tips: boolean, seenTips: Array<string> }`, `SettingsPatch = { locale?, theme?, lastProject?, tips?: boolean, seenTips?: Array<string> }`. Патч заменяет `seenTips` целиком.

- [ ] **Step 1: Написать падающие тесты**

В конец `app/core/tests/settings.rs`:

```rust
#[test]
fn подсказки_по_умолчанию_включены_и_не_просмотрены() {
    let (dir, settings) = settings();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    std::fs::write(
        dir.path().join("data").join("settings.json"),
        "{\"locale\":\"en\"}",
    )
    .unwrap();
    let got = settings.get().unwrap();
    assert!(got.tips);
    assert!(got.seen_tips.is_empty());
}

#[test]
fn подсказки_сохраняются_и_читаются_заново() {
    let (dir, settings) = settings();
    settings
        .update(SettingsPatch {
            tips: Some(false),
            seen_tips: Some(vec!["report.hotspots".into(), "add-project.input".into()]),
            ..Default::default()
        })
        .unwrap();
    let again = Settings::new(dir.path().join("data").join("settings.json"));
    let got = again.get().unwrap();
    assert!(!got.tips);
    assert_eq!(got.seen_tips, vec!["report.hotspots", "add-project.input"]);
}
```

- [ ] **Step 2: Убедиться, что не компилируется**

Run: `cargo test -p longpole-core --test settings`
Expected: FAIL — `no field 'tips' on type 'AppSettings'`, `struct 'SettingsPatch' has no field named 'tips'`.

- [ ] **Step 3: Поля в `settings.rs`**

В `Stored` после `last_project`:

```rust
    tips: Option<bool>,
    seen_tips: Option<Vec<String>>,
```

`AppSettings` целиком (Copy нет, `Vec` допустим):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppSettings {
    pub locale: Locale,
    pub theme: Theme,
    pub last_project: Option<ProjectRef>,
    /// показывать подсказки онбординга
    pub tips: bool,
    /// id подсказок, которые уже показаны
    pub seen_tips: Vec<String>,
}
```

В `SettingsPatch` после `last_project`:

```rust
    #[ts(optional)]
    pub tips: Option<bool>,
    /// заменяет список целиком
    #[ts(optional)]
    pub seen_tips: Option<Vec<String>>,
```

В `get()` в литерал `AppSettings` после `last_project: stored.last_project,`:

```rust
            tips: stored.tips.unwrap_or(true),
            seen_tips: stored.seen_tips.unwrap_or_default(),
```

В `update()` после блока `last_project`:

```rust
        if patch.tips.is_some() {
            stored.tips = patch.tips;
        }
        if patch.seen_tips.is_some() {
            stored.seen_tips = patch.seen_tips;
        }
```

В строке-комментарии модуля (строка 1) дописать: `…, подсказки онбординга.`

- [ ] **Step 4: Починить литерал в старом тесте**

В `app/core/tests/settings.rs`, тест `патч_меняет_только_переданное…`, литерал `AppSettings { locale: Locale::system(), theme: Theme::Dark, last_project: Some(project.clone()), }` дополнить:

```rust
            tips: true,
            seen_tips: vec![],
```

- [ ] **Step 5: Тесты зелёные, схема перегенерирована**

Run: `cargo fmt --all && cargo test -p longpole-core`
Expected: PASS; `git status` показывает изменённые `src/shared/api/schema/AppSettings.ts` и `SettingsPatch.ts` с `tips` и `seenTips`.

- [ ] **Step 6: Мок dev-моста**

`app/src/app/dev/mock.ts:12`:

```ts
let settings: AppSettings = { locale: 'ru', theme: 'system', lastProject: { host: HOST, path: 'g/p' }, tips: true, seenTips: [] }
```

Run: `npm run typecheck`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add app/core/src/settings.rs app/core/tests/settings.rs app/src/shared/api/schema app/src/app/dev/mock.ts
git commit -m "feat: настройки tips и seenTips для онбординга"
```

---

### Task 2: Жираф — компонент, токены, пустые состояния, загрузка, ошибки

**Files:**
- Create: `app/src/shared/ui/giraffe.tsx`
- Modify: `app/src/shared/ui/theme.css` (токены после `--color-crit-bg` в `@theme static` и в `:root.dark`, анимации, правило `[data-tip-active]`)
- Modify: `app/src/shared/ui/paged-list.tsx` (проп `empty`)
- Modify: `app/src/widgets/pipelines-list/ui/PipelinesList.tsx`
- Modify: `app/src/pages/home/ui/HomePage.tsx:25-26`
- Modify: `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx:86`
- Modify: `app/src/entities/history-entry/model/useBuild.ts:29-35`
- Modify: `app/src/widgets/aggregate-block/ui/AggregateBlock.tsx`
- Modify: `app/src/app/form/report-route.tsx:42`
- Modify: `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx:64-68`
- Modify: `app/src/features/manage-token/ui/TokenBlock.tsx:46-50`

**Interfaces:**
- Produces: `Giraffe({ pose?: 'idle' | 'loading' | 'oops', size?: number, progress?: { loaded: number; total: number }, className?: string })`, `type GiraffePose`.
- Produces: Tailwind-утилиты `text-mascot`, `bg-mascot-bg`, `border-mascot-border`, `animate-tip-in`, `animate-giraffe-bar`; CSS-правило для `[data-tip-active]`.
- Produces: `useBuild()` возвращает ещё `progress: Progress | null`; `useBuildAggregate` прокидывает его спредом.
- Produces: `PagedList` принимает `empty?: ReactNode` — рисуется вместо «Ничего не найдено».

Автотестов у фронтенда нет (кроме задачи 3); критерий — `npm run typecheck && npm run lint:fsd && npm run check:locales` и просмотр в `npm run dev`.

- [ ] **Step 1: Токены и анимации в `theme.css`**

В `@theme static` сразу после строки `--color-crit-bg: #fff7eb;`:

```css
  /* маскот-жираф: силуэт, подложка и рамка подсказки (спека онбординга, § 3) */
  --color-mascot: #e8701a;
  --color-mascot-bg: #fff4e8;
  --color-mascot-border: #f2c79c;

  --animate-tip-in: tip-in 150ms ease-out;
  --animate-giraffe-bar: giraffe-bar 1.2s steps(1) infinite;

  @keyframes tip-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
  }
  /* полоска шеи горит четверть цикла; задержки 0/300/600/900 мс дают бегущий огонёк */
  @keyframes giraffe-bar {
    0% {
      opacity: 1;
    }
    25%,
    100% {
      opacity: 0.2;
    }
  }
```

В `:root.dark` сразу после строки `--color-crit-bg: #2a2416;`:

```css
  --color-mascot: #ff8a3d;
  --color-mascot-bg: #2e2216;
  --color-mascot-border: #5a3d10;
```

После `@utility focus-ring { … }` (строка ~142), до `@layer base`:

```css
/* якорь активной подсказки онбординга */
@layer base {
  [data-tip-active] {
    outline: 2px solid var(--color-mascot);
    outline-offset: 2px;
  }
}
```

- [ ] **Step 2: Компонент `giraffe.tsx`**

```tsx
import { cn } from '../lib/cn'

export type GiraffePose = 'idle' | 'loading' | 'oops'

// шея — полоски трейса снизу вверх, геометрия иконки `app/src-tauri/icons/app-icon.svg` без подложки
const NECK = [
  { x: 18, y: 78, width: 30, opacity: 0.55 },
  { x: 26, y: 67, width: 28, opacity: 0.7 },
  { x: 34, y: 56, width: 26, opacity: 0.85 },
  { x: 42, y: 45, width: 22, opacity: 1 },
]
const EYE = '#1c2333'

type Props = { pose?: GiraffePose; size?: number; progress?: { loaded: number; total: number }; className?: string }

/** Маскот с иконки приложения. Декоративный: смысл несёт текст рядом. `loading` с `progress` зажигает полоски шеи по доле загруженного. */
export function Giraffe({ pose = 'idle', size = 56, progress, className }: Props) {
  const lit = pose === 'loading' && progress && progress.total > 0 ? Math.ceil((NECK.length * progress.loaded) / progress.total) : null
  const cycling = pose === 'loading' && lit === null
  return (
    <svg viewBox="14 8 72 82" width={size} height={Math.round((size * 82) / 72)} aria-hidden className={cn('shrink-0 text-mascot', className)}>
      <g fill="currentColor">
        {NECK.map((bar, i) => (
          <rect
            key={bar.y}
            x={bar.x}
            y={bar.y}
            width={bar.width}
            height={9}
            rx={3}
            opacity={lit !== null ? (i < lit ? 1 : 0.2) : cycling ? 0.2 : bar.opacity}
            className={cycling ? 'animate-giraffe-bar motion-reduce:animate-none' : undefined}
            style={cycling ? { animationDelay: `${i * 300}ms` } : undefined}
          />
        ))}
        {/* `oops` опускает голову: поворот вокруг верха шеи */}
        <g transform={pose === 'oops' ? 'rotate(20 53 44)' : undefined}>
          <path d="M44 42 C44 33 50 28 58 28 L76 33 C83 35 84 44 77 46 L60 47 C52 48 46 47 44 42 Z" />
          <circle cx="48" cy="17" r="3.6" />
          <circle cx="57" cy="16" r="3.6" />
          <ellipse cx="44" cy="31" rx="6" ry="3" transform="rotate(-25 44 31)" />
          <path d="M50 30 L48 18 M57 29 L57 17" fill="none" stroke="currentColor" strokeWidth="4" strokeLinecap="round" />
          {pose === 'oops' ? (
            <path d="M59.5 35 q2.6 2 5.2 0" fill="none" stroke={EYE} strokeWidth="1.6" strokeLinecap="round" />
          ) : (
            <circle cx="62" cy="35" r="2.6" fill={EYE} />
          )}
        </g>
      </g>
    </svg>
  )
}
```

- [ ] **Step 3: Пустой главный экран и «Недавние»**

`HomePage.tsx` — импорт `import { Giraffe } from '../../../shared/ui/giraffe'`, в карточке пустого состояния перед `<h1>`:

```tsx
        <Giraffe size={120} />
```

`ProjectsSidebar.tsx:86` — импорт `Giraffe` из `../../../shared/ui/giraffe`, строку заменить:

```tsx
        {entries.length === 0 && !historyError && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Giraffe size={32} />
            {t('form.history.empty')}
          </p>
        )}
```

- [ ] **Step 4: Пустой список пайплайнов**

`paged-list.tsx`: тип пропсов и начало тела.

```tsx
type Props<T> = {
  query: UseInfiniteQueryResult<InfiniteData<Page<T>>>
  children: (item: T) => ReactNode
  /** вместо «Ничего не найдено», когда список пуст */
  empty?: ReactNode
}

/** Список постраничного запроса: «Загрузка…», «Ничего не найдено» (или `empty`), ошибка над списком и «Показать ещё». */
export function PagedList<T>({ query, children, empty }: Props<T>) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const failure = apiError(query.error)
  const nothing = !query.isPending && !query.isFetchingNextPage && !query.isError && items.length === 0
  const status = query.isPending || query.isFetchingNextPage ? t('form.loading') : nothing ? t('form.nothing') : ''
  return (
    <div className="flex flex-col gap-2">
      {nothing && empty ? (
        empty
      ) : (
        <p aria-live="polite" className="text-sm text-muted-foreground empty:hidden">
          {status}
        </p>
      )}
```

Остальная разметка (`query.isError`, `<ul>`, «Показать ещё») без изменений.

`PipelinesList.tsx` — импорты `useTranslation` из `../../../shared/i18n` и `Giraffe` из `../../../shared/ui/giraffe`; функция:

```tsx
export function PipelinesList({ host, project, branch, workflow }: ProjectRef) {
  const { t } = useTranslation()
  const query = usePipelines(host, project, branch, workflow)
  const empty = (
    <p aria-live="polite" className="flex items-center gap-3 text-sm text-muted-foreground">
      <Giraffe size={72} />
      {t('form.nothing')}
    </p>
  )
  return (
    <PagedList query={query} empty={empty}>
      {(pipeline) => <BuildRow key={pipeline.id} pipeline={pipeline} />}
    </PagedList>
  )
}
```

- [ ] **Step 5: Прогресс сборки наружу из `useBuild`**

`useBuild.ts`, возвращаемый объект:

```ts
  return {
    start: mutation.mutate,
    busy: mutation.isPending,
    /** сырой прогресс для жирафа загрузки; `null`, пока ядро его не прислало */
    progress,
    /** «Загружаю N из M…» (или «Загружаю…», пока прогресса нет). */
    progressLabel: progress ? t('form.progress.step', progress) : t('form.progress.start'),
    error: apiError(mutation.error),
  }
```

`useBuildAggregate` прокидывает `...build` — правка не нужна.

- [ ] **Step 6: Жираф загрузки у агрегата и на экране загрузки отчёта**

`AggregateBlock.tsx` — импорт `Giraffe` из `../../../shared/ui/giraffe`; деструктуризация `const { start, busy, progress, progressLabel, error } = useBuildAggregate(target)`; строку с `<BuildAggregateButton … />` заменить:

```tsx
      <div className="flex items-end gap-3">
        <BuildAggregateButton busy={busy} progressLabel={progressLabel} onClick={start} />
        {busy && <Giraffe pose="loading" size={72} progress={progress ?? undefined} />}
      </div>
```

`report-route.tsx:42` — импорт `Giraffe` из `../../shared/ui/giraffe`; строку заменить:

```tsx
  if (isPending || !data) {
    return (
      <p className="flex items-center gap-3 p-6 text-sm text-muted-foreground">
        <Giraffe pose="loading" size={72} />
        {t('form.reportView.loading')}
      </p>
    )
  }
```

- [ ] **Step 7: Жираф `oops` в inline-ошибках**

`AddProjectDialog.tsx:64-68` (импорт `Giraffe` из `../../../shared/ui/giraffe`):

```tsx
        {showError && (
          <p id={`${id}-error`} role="alert" className="flex items-start gap-2 text-sm text-destructive">
            <Giraffe pose="oops" size={40} />
            {errorText(failure)}
          </p>
        )}
```

`TokenBlock.tsx:46-50` (импорт `Giraffe` из `../../../shared/ui/giraffe`):

```tsx
      {failure && (
        <p id={`${id}-error`} role="alert" className="flex items-start gap-2 text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
```

- [ ] **Step 8: Проверка**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: PASS.

Run: `npm run dev`, открыть в браузере. Проверить:
- пустое состояние главного экрана — удалить оба проекта крестиком в сайдбаре;
- «Недавние» — кнопка «Очистить»;
- агрегат — жираф с бегущими полосками (мок не шлёт прогресс);
- диалог «Добавить проект» с вводом `мусор` — жираф `oops` под полем;
- тёмная тема в настройках — силуэт `#ff8a3d`;
- в DevTools `prefers-reduced-motion: reduce` — полоски не бегут.

- [ ] **Step 9: Commit**

```bash
git add app/src/shared/ui app/src/widgets app/src/pages/home app/src/entities/history-entry app/src/app/form/report-route.tsx app/src/features/manage-token
git commit -m "feat: маскот-жираф в пустых состояниях, загрузке и ошибках"
```

---

### Task 3: Онбординг — `pickTip` с тестом, реестр, стор

**Files:**
- Create: `app/src/features/onboarding/model/pick.ts`
- Create: `app/src/features/onboarding/model/pick.test.ts`
- Create: `app/src/features/onboarding/model/tips.ts`
- Create: `app/src/features/onboarding/model/store.ts`
- Modify: `app/package.json` (скрипт `test:onboarding`)
- Modify: `lefthook.yml` (pre-commit `test-onboarding`)
- Modify: `.github/workflows/ci.yml` (шаг после `check:locales`)

**Interfaces:**
- Consumes: `SettingsPatch` из `shared/api` (задача 1).
- Produces: `pickTip<Id>(input: PickInput<Id>): Id | null`, `type TipDef<Id>`, `TIPS`, `type TipId`, стор: `hydrate(enabled: boolean, seen: readonly string[])`, `setPersist(fn: (patch: SettingsPatch) => void)`, `addReady(id: TipId)`, `removeReady(id: TipId)`, `closeTip()`, `disableTips()`, `startVisit()`, `markLeftReport()`, `useTipOpen(id: TipId): boolean`, `useLeftReport(): boolean`.

`pick.ts` без импортов: `node --test` запускает его без сборки (Node ≥ 22.18 снимает типы сам; локально Node 24, CI — последняя 22.x).

- [ ] **Step 1: Падающий тест**

`pick.test.ts`:

```ts
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { pickTip, type TipDef } from './pick.ts'

const ORDER: TipDef[] = [
  { id: 'event', kind: 'event' },
  { id: 'first', kind: 'enter' },
  { id: 'second', kind: 'enter' },
  { id: 'chained', kind: 'enter', after: 'first' },
]
const base = { order: ORDER, ready: [] as string[], seen: [] as string[], enabled: true, enterShown: false, active: null as string | null }

test('две enter готовы — выбрана первая по реестру', () => {
  assert.equal(pickTip({ ...base, ready: ['second', 'first'] }), 'first')
})

test('enter уже была за визит — следующая enter не выбирается', () => {
  assert.equal(pickTip({ ...base, ready: ['second'], enterShown: true }), null)
})

test('event выбирается после enter за визит и раньше готовой enter', () => {
  assert.equal(pickTip({ ...base, ready: ['event'], enterShown: true }), 'event')
  assert.equal(pickTip({ ...base, ready: ['first', 'event'] }), 'event')
})

test('открытая подсказка блокирует выбор', () => {
  assert.equal(pickTip({ ...base, ready: ['first'], active: 'event' }), null)
})

test('after не просмотрена — подсказка ждёт', () => {
  assert.equal(pickTip({ ...base, ready: ['chained'] }), null)
  assert.equal(pickTip({ ...base, ready: ['chained'], seen: ['first'] }), 'chained')
})

test('просмотренная не выбирается', () => {
  assert.equal(pickTip({ ...base, ready: ['first'], seen: ['first'] }), null)
})

test('подсказки выключены — ничего', () => {
  assert.equal(pickTip({ ...base, ready: ['event', 'first'], enabled: false }), null)
})
```

В `app/package.json`, `scripts`, после `check:locales`:

```json
    "test:onboarding": "node --test src/features/onboarding/model/pick.test.ts",
```

Run: `npm run test:onboarding`
Expected: FAIL — `Cannot find module …/pick.ts`.

- [ ] **Step 2: `pick.ts`**

```ts
/** Выбор подсказки — чистая функция без импортов: её гоняет `node --test` без сборки (спека онбординга, § 4). */

export type TipDef<Id extends string = string> = {
  id: Id
  /** `enter` — готова, пока открыт экран или элемент; `event` — в момент действия пользователя */
  kind: 'enter' | 'event'
  /** показывается только после этой */
  after?: Id
}

export type PickInput<Id extends string> = {
  /** реестр в порядке приоритета */
  order: readonly TipDef<Id>[]
  ready: readonly Id[]
  seen: readonly string[]
  enabled: boolean
  /** `enter` за этот визит экрана уже была */
  enterShown: boolean
  active: Id | null
}

/** `event` важнее `enter`: она отвечает на действие прямо сейчас; `enter` — не больше одной за визит. */
export function pickTip<Id extends string>({ order, ready, seen, enabled, enterShown, active }: PickInput<Id>): Id | null {
  if (!enabled || active !== null) return null
  const candidates = order.filter((tip) => ready.includes(tip.id) && !seen.includes(tip.id) && (tip.after === undefined || seen.includes(tip.after)))
  const event = candidates.find((tip) => tip.kind === 'event')
  if (event) return event.id
  if (enterShown) return null
  return candidates.find((tip) => tip.kind === 'enter')?.id ?? null
}
```

Run: `npm run test:onboarding`
Expected: PASS, 7 тестов.

- [ ] **Step 3: Реестр `tips.ts`**

```ts
import type { TipDef } from './pick'

/** Каталог подсказок (спека онбординга, § 2). Порядок — приоритет; тексты — `onboarding.<id>.title|text`. */
export const TIPS = [
  { id: 'add-project.input', kind: 'event' },
  { id: 'token.create', kind: 'event' },
  { id: 'header.link', kind: 'event' },
  { id: 'report.detail', kind: 'event' },
  { id: 'project.screen', kind: 'enter' },
  { id: 'project.workflow', kind: 'enter', after: 'project.screen' },
  { id: 'report.hotspots', kind: 'enter' },
  { id: 'report.critical', kind: 'enter', after: 'report.hotspots' },
  { id: 'report.keys', kind: 'enter', after: 'report.critical' },
  { id: 'report.retries', kind: 'enter' },
  { id: 'report.aggregate', kind: 'enter' },
] as const satisfies readonly TipDef[]

export type TipId = (typeof TIPS)[number]['id']
```

- [ ] **Step 4: Стор `store.ts`**

```ts
import { create } from 'zustand'
import type { SettingsPatch } from '../../../shared/api'
import { pickTip } from './pick'
import { TIPS, type TipId } from './tips'

type State = {
  /** до `hydrate` выключено: в сохранённом HTML-отчёте настроек нет, и подсказки там не показываются */
  enabled: boolean
  seen: readonly string[]
  ready: readonly TipId[]
  active: TipId | null
  enterShown: boolean
  leftReport: boolean
}

const useStore = create<State>(() => ({ enabled: false, seen: [], ready: [], active: null, enterShown: false, leftReport: false }))

// запись в settings.json ставит `OnboardingSync`: стор лежит ниже react-query и не знает о нём
let persist: (patch: SettingsPatch) => void = () => {}
export const setPersist = (fn: (patch: SettingsPatch) => void) => {
  persist = fn
}

/** Показанная подсказка сразу просмотрена: быстрое закрытие диалога не даёт повтора. */
function repick() {
  const s = useStore.getState()
  const id = pickTip({ order: TIPS, ready: s.ready, seen: s.seen, enabled: s.enabled, enterShown: s.enterShown, active: s.active })
  if (id === null) return
  const seen = [...s.seen, id]
  const enter = TIPS.find((tip) => tip.id === id)?.kind === 'enter'
  useStore.setState({ active: id, seen, enterShown: s.enterShown || enter })
  persist({ seenTips: seen })
}

export function hydrate(enabled: boolean, seen: readonly string[]) {
  useStore.setState((s) => ({ enabled, seen, active: enabled ? s.active : null }))
  repick()
}

export function addReady(id: TipId) {
  useStore.setState((s) => (s.ready.includes(id) ? s : { ready: [...s.ready, id] }))
  repick()
}

/** Активная уходит через микротаск: StrictMode в dev сразу монтирует эффект заново, и подсказка не должна гаснуть. */
export function removeReady(id: TipId) {
  useStore.setState((s) => ({ ready: s.ready.filter((r) => r !== id) }))
  queueMicrotask(() => {
    const s = useStore.getState()
    if (s.active !== id || s.ready.includes(id)) return
    useStore.setState({ active: null })
    repick()
  })
}

export function closeTip() {
  useStore.setState({ active: null })
  repick()
}

export function disableTips() {
  useStore.setState({ enabled: false, active: null })
  persist({ tips: false })
}

/** Новый визит экрана: снова можно одну `enter`. */
export function startVisit() {
  useStore.setState({ enterShown: false })
  repick()
}

export function markLeftReport() {
  useStore.setState({ leftReport: true })
}

export const useTipOpen = (id: TipId) => useStore((s) => s.active === id)
export const useLeftReport = () => useStore((s) => s.leftReport)
```

- [ ] **Step 5: Тест в хуки и CI**

`lefthook.yml`, `pre-commit.commands`, после `check-locales`:

```yaml
    test-onboarding:
      root: app/
      glob: 'app/src/features/onboarding/**/*.ts'
      run: npm run test:onboarding
```

`.github/workflows/ci.yml`, после `- run: npm run check:locales`:

```yaml
      - run: npm run test:onboarding
```

- [ ] **Step 6: Проверка**

Run: `npm run typecheck && npm run lint:fsd && npm run test:onboarding`
Expected: PASS. Если Steiger ругается на `pick.test.ts` в сегменте `model` — добавить исключение для `**/*.test.ts` в `app/steiger.config.ts` по образцу существующих.

- [ ] **Step 7: Commit**

```bash
git add app/src/features/onboarding app/package.json lefthook.yml .github/workflows/ci.yml
git commit -m "feat: выбор подсказки онбординга, реестр и стор"
```

---

### Task 4: Онбординг — `<Tip>`, пузырь, синхронизация, блок настроек, словари

**Files:**
- Create: `app/src/features/onboarding/ui/Tip.tsx`
- Create: `app/src/features/onboarding/ui/TipBubble.tsx`
- Create: `app/src/features/onboarding/ui/OnboardingSync.tsx`
- Create: `app/src/features/onboarding/ui/TipsSettings.tsx`
- Create: `app/src/features/onboarding/model/useOnboardingVisit.ts`
- Create: `app/src/features/onboarding/index.ts`
- Modify: `app/src/app/form/main.tsx` (монтирование `OnboardingSync`)
- Modify: `app/src/pages/settings/ui/SettingsPage.tsx:73` (блок «Подсказки» между темой и языком)
- Modify: `app/src/shared/i18n/locales/{ru,en,fr,es,de,it,zh,ja}.json`

**Interfaces:**
- Consumes: стор и `TipId` (задача 3), `Giraffe` (задача 2), `useSettings`/`useUpdateSettings` из `entities/settings`, `Popover`/`PopoverAnchor`/`PopoverContent` из `shared/ui/popover`.
- Produces (публичный API слайса): `Tip({ id: TipId, when?: boolean, children: ReactElement })`, `OnboardingSync()`, `TipsSettings()`, `useOnboardingVisit()`, `markLeftReport()`, `useLeftReport()`, `type TipId`.
- Ребёнок `<Tip>` — один элемент, который передаёт `ref` и пропсы в DOM (`<div>`, `<Input>`, `<Card>`): `PopoverAnchor asChild` вешает на него ref и `data-tip-active`. Компонент, который этого не делает (`TokenBlock`, `KeysHelp`, `BuildAggregateButton`, `WorkflowSelect`, `BuildByLink`, `PanelHead`), оборачивается в `<div>`/`<span>`.

- [ ] **Step 1: Словари**

В каждый из 8 файлов `app/src/shared/i18n/locales/*.json`: в объект `form.settings` — три ключа, на верхний уровень после `report` — объект `onboarding`. Тексты ниже — финальные; перед вставкой сверить термины («агрегат», «критический путь», «стейдж», «ретраи») с уже существующими ключами `form.aggregate.*` и `report.*` того же файла и при расхождении взять термин файла.

`ru.json`:

```json
"tips": "Подсказки",
"tipsShow": "Показывать подсказки жирафа",
"tipsReset": "Показать заново"
```

```json
"onboarding": {
  "ok": "Понятно",
  "never": "Больше не показывать",
  "add-project": { "input": { "title": "Что сюда вставить", "text": "Подойдёт <b>ссылка на репозиторий, пайплайн или MR</b>, ssh-адрес или папка с клоном — кнопка справа." } },
  "token": { "create": { "title": "Нужен токен", "text": "Ссылка откроет страницу создания токена <b>с нужными правами</b>. Токен хранится в системной связке ключей." } },
  "project": {
    "screen": { "title": "Один пайплайн или много", "text": "Клик по пайплайну строит отчёт по нему. <b>Агрегат</b> собирает N пайплайнов ветки и показывает p50 и p90." },
    "workflow": { "title": "Workflow", "text": "У GitHub запуски и агрегат строятся <b>по одному workflow</b> — выбери его здесь." }
  },
  "header": { "link": { "title": "Можно короче", "text": "В следующий раз вставь сюда <b>ссылку на пайплайн, MR, run или PR</b> — отчёт построится без захода в проект." } },
  "report": {
    "hotspots": { "title": "С чего начать", "text": "Здесь только джобы <b>критического пути</b>: ускорение остальных пайплайн не сократит. Клик ведёт к джобе." },
    "critical": { "title": "Оранжевое — критический путь", "text": "Эти джобы определяют длину пайплайна. <b>Клик по стейджу</b> пересчитывает путь до его конца." },
    "keys": { "title": "Масштаб и клавиши", "text": "<b>⌘/Ctrl + колесо</b> — масштаб, <b>[</b> и <b>]</b> — сдвиг, <b>0</b> — весь пайплайн. Остальное — под «?»." },
    "retries": { "title": "Ретраи", "text": "Красная штриховка — упавшие попытки. Подпись <b>↻2 −8m29s</b> — сколько их было и сколько времени на них ушло." },
    "aggregate": { "title": "Как читать агрегат", "text": "Полоска — <b>p50</b> старта и длительности, линия после неё тянется до <b>p90</b>. Метка справа — доля запусков с ретраями." },
    "detail": { "title": "Панель деталей", "text": "Сверху — <b>вывод</b>: сократит ли ускорение пайплайн. Ниже связи ↑/↓. Esc снимает выбор." }
  }
}
```

`en.json`:

```json
"tips": "Tips",
"tipsShow": "Show giraffe tips",
"tipsReset": "Show again"
```

```json
"onboarding": {
  "ok": "Got it",
  "never": "Don't show tips",
  "add-project": { "input": { "title": "What goes here", "text": "A <b>link to a repository, pipeline or MR</b>, an ssh address, or a folder with a clone — the button on the right." } },
  "token": { "create": { "title": "A token is needed", "text": "The link opens the token page <b>with the right scopes</b> preset. The token is kept in the system keychain." } },
  "project": {
    "screen": { "title": "One pipeline or many", "text": "Click a pipeline to build its report. The <b>aggregate</b> combines N pipelines of the branch and shows p50 and p90." },
    "workflow": { "title": "Workflow", "text": "On GitHub, runs and the aggregate are built <b>for one workflow</b> — pick it here." }
  },
  "header": { "link": { "title": "A shortcut", "text": "Next time, paste a <b>pipeline, MR, run or PR link</b> here — the report builds without opening the project." } },
  "report": {
    "hotspots": { "title": "Where to start", "text": "Only <b>critical path</b> jobs are listed: speeding up others won't shorten the pipeline. Click to jump to the job." },
    "critical": { "title": "Orange is the critical path", "text": "These jobs set the pipeline length. <b>Click a stage</b> to recompute the path up to its end." },
    "keys": { "title": "Zoom and keys", "text": "<b>⌘/Ctrl + wheel</b> zooms, <b>[</b> and <b>]</b> pan, <b>0</b> shows the whole pipeline. The rest is under “?”." },
    "retries": { "title": "Retries", "text": "Red hatching marks failed attempts. The label <b>↻2 −8m29s</b> shows how many there were and the time lost on them." },
    "aggregate": { "title": "Reading the aggregate", "text": "The bar is the <b>p50</b> start and duration, the line after it reaches <b>p90</b>. The label on the right is the share of runs with retries." },
    "detail": { "title": "Details panel", "text": "At the top is the <b>verdict</b>: whether speeding this up shortens the pipeline. Links ↑/↓ are below. Esc clears the selection." }
  }
}
```

`fr.json`:

```json
"tips": "Astuces",
"tipsShow": "Afficher les astuces de la girafe",
"tipsReset": "Réafficher"
```

```json
"onboarding": {
  "ok": "Compris",
  "never": "Ne plus afficher",
  "add-project": { "input": { "title": "Que mettre ici", "text": "Un <b>lien vers un dépôt, un pipeline ou une MR</b>, une adresse ssh ou un dossier avec un clone — bouton à droite." } },
  "token": { "create": { "title": "Un jeton est nécessaire", "text": "Le lien ouvre la page de création du jeton <b>avec les bons droits</b>. Le jeton est conservé dans le trousseau du système." } },
  "project": {
    "screen": { "title": "Un pipeline ou plusieurs", "text": "Un clic sur un pipeline construit son rapport. L'<b>agrégat</b> réunit N pipelines de la branche et montre p50 et p90." },
    "workflow": { "title": "Workflow", "text": "Sur GitHub, les exécutions et l'agrégat portent <b>sur un seul workflow</b> : choisissez-le ici." }
  },
  "header": { "link": { "title": "Plus court", "text": "La prochaine fois, collez ici un <b>lien vers un pipeline, une MR, un run ou une PR</b> : le rapport se construit sans ouvrir le projet." } },
  "report": {
    "hotspots": { "title": "Par où commencer", "text": "Seuls les jobs du <b>chemin critique</b> figurent ici : accélérer les autres ne raccourcit pas le pipeline. Un clic mène au job." },
    "critical": { "title": "L'orange, c'est le chemin critique", "text": "Ces jobs fixent la durée du pipeline. <b>Un clic sur un stage</b> recalcule le chemin jusqu'à sa fin." },
    "keys": { "title": "Zoom et touches", "text": "<b>⌘/Ctrl + molette</b> zoome, <b>[</b> et <b>]</b> décalent, <b>0</b> montre tout le pipeline. Le reste est sous « ? »." },
    "retries": { "title": "Relances", "text": "Les hachures rouges marquent les tentatives échouées. L'étiquette <b>↻2 −8m29s</b> indique leur nombre et le temps perdu." },
    "aggregate": { "title": "Lire l'agrégat", "text": "La barre montre le début et la durée <b>p50</b>, la ligne qui suit va jusqu'au <b>p90</b>. L'étiquette à droite indique la part d'exécutions avec relances." },
    "detail": { "title": "Panneau de détails", "text": "En haut, la <b>conclusion</b> : accélérer ce job raccourcit-il le pipeline ? Les liens ↑/↓ sont dessous. Échap annule la sélection." }
  }
}
```

`es.json`:

```json
"tips": "Consejos",
"tipsShow": "Mostrar los consejos de la jirafa",
"tipsReset": "Volver a mostrar"
```

```json
"onboarding": {
  "ok": "Entendido",
  "never": "No volver a mostrar",
  "add-project": { "input": { "title": "Qué poner aquí", "text": "Sirve un <b>enlace a un repositorio, pipeline o MR</b>, una dirección ssh o una carpeta con un clon (botón a la derecha)." } },
  "token": { "create": { "title": "Hace falta un token", "text": "El enlace abre la página de creación del token <b>con los permisos necesarios</b>. El token se guarda en el llavero del sistema." } },
  "project": {
    "screen": { "title": "Un pipeline o muchos", "text": "Al hacer clic en un pipeline se crea su informe. El <b>agregado</b> reúne N pipelines de la rama y muestra p50 y p90." },
    "workflow": { "title": "Workflow", "text": "En GitHub, las ejecuciones y el agregado se construyen <b>para un solo workflow</b>: elígelo aquí." }
  },
  "header": { "link": { "title": "Un atajo", "text": "La próxima vez, pega aquí un <b>enlace a un pipeline, MR, run o PR</b>: el informe se crea sin abrir el proyecto." } },
  "report": {
    "hotspots": { "title": "Por dónde empezar", "text": "Aquí solo hay jobs de la <b>ruta crítica</b>: acelerar los demás no acorta el pipeline. Haz clic para ir al job." },
    "critical": { "title": "Lo naranja es la ruta crítica", "text": "Estos jobs fijan la duración del pipeline. <b>Haz clic en un stage</b> para recalcular la ruta hasta su final." },
    "keys": { "title": "Zoom y teclas", "text": "<b>⌘/Ctrl + rueda</b> para el zoom, <b>[</b> y <b>]</b> para desplazar, <b>0</b> muestra todo el pipeline. El resto, en «?»." },
    "retries": { "title": "Reintentos", "text": "El rayado rojo marca los intentos fallidos. La etiqueta <b>↻2 −8m29s</b> indica cuántos hubo y el tiempo perdido." },
    "aggregate": { "title": "Cómo leer el agregado", "text": "La barra es el inicio y la duración <b>p50</b>; la línea que sigue llega al <b>p90</b>. La etiqueta de la derecha es la proporción de ejecuciones con reintentos." },
    "detail": { "title": "Panel de detalles", "text": "Arriba está la <b>conclusión</b>: si acelerarlo acorta el pipeline. Debajo, los vínculos ↑/↓. Esc quita la selección." }
  }
}
```

`de.json`:

```json
"tips": "Tipps",
"tipsShow": "Tipps der Giraffe anzeigen",
"tipsReset": "Erneut zeigen"
```

```json
"onboarding": {
  "ok": "Verstanden",
  "never": "Nicht mehr anzeigen",
  "add-project": { "input": { "title": "Was hier hineingehört", "text": "Ein <b>Link auf Repository, Pipeline oder MR</b>, eine ssh-Adresse oder ein Ordner mit einem Klon – Schaltfläche rechts." } },
  "token": { "create": { "title": "Ein Token wird benötigt", "text": "Der Link öffnet die Token-Seite <b>mit den nötigen Rechten</b>. Das Token liegt im Schlüsselbund des Systems." } },
  "project": {
    "screen": { "title": "Eine Pipeline oder viele", "text": "Ein Klick auf eine Pipeline erstellt ihren Bericht. Das <b>Aggregat</b> fasst N Pipelines des Branches zusammen und zeigt p50 und p90." },
    "workflow": { "title": "Workflow", "text": "Bei GitHub beziehen sich Läufe und Aggregat <b>auf einen Workflow</b> – wähle ihn hier." }
  },
  "header": { "link": { "title": "Abkürzung", "text": "Füge beim nächsten Mal hier einen <b>Link auf Pipeline, MR, Run oder PR</b> ein – der Bericht entsteht ohne Umweg über das Projekt." } },
  "report": {
    "hotspots": { "title": "Wo anfangen", "text": "Hier stehen nur Jobs des <b>kritischen Pfads</b>: Andere zu beschleunigen verkürzt die Pipeline nicht. Ein Klick springt zum Job." },
    "critical": { "title": "Orange ist der kritische Pfad", "text": "Diese Jobs bestimmen die Pipeline-Dauer. <b>Klick auf eine Stage</b> berechnet den Pfad bis zu ihrem Ende neu." },
    "keys": { "title": "Zoom und Tasten", "text": "<b>⌘/Strg + Mausrad</b> zoomt, <b>[</b> und <b>]</b> verschieben, <b>0</b> zeigt die ganze Pipeline. Der Rest steht unter „?“." },
    "retries": { "title": "Wiederholungen", "text": "Rote Schraffur markiert fehlgeschlagene Versuche. Die Beschriftung <b>↻2 −8m29s</b> zeigt ihre Anzahl und die verlorene Zeit." },
    "aggregate": { "title": "Das Aggregat lesen", "text": "Der Balken zeigt Start und Dauer im <b>p50</b>, die Linie danach reicht bis <b>p90</b>. Die Markierung rechts ist der Anteil der Läufe mit Wiederholungen." },
    "detail": { "title": "Detailbereich", "text": "Oben steht das <b>Fazit</b>: ob eine Beschleunigung die Pipeline verkürzt. Darunter die Beziehungen ↑/↓. Esc hebt die Auswahl auf." }
  }
}
```

`it.json`:

```json
"tips": "Suggerimenti",
"tipsShow": "Mostra i suggerimenti della giraffa",
"tipsReset": "Mostra di nuovo"
```

```json
"onboarding": {
  "ok": "Capito",
  "never": "Non mostrare più",
  "add-project": { "input": { "title": "Cosa inserire qui", "text": "Va bene un <b>link a un repository, pipeline o MR</b>, un indirizzo ssh o una cartella con un clone (pulsante a destra)." } },
  "token": { "create": { "title": "Serve un token", "text": "Il link apre la pagina di creazione del token <b>con i permessi giusti</b>. Il token resta nel portachiavi di sistema." } },
  "project": {
    "screen": { "title": "Una pipeline o molte", "text": "Un clic su una pipeline ne crea il report. L'<b>aggregato</b> unisce N pipeline del branch e mostra p50 e p90." },
    "workflow": { "title": "Workflow", "text": "Su GitHub, le esecuzioni e l'aggregato riguardano <b>un solo workflow</b>: sceglilo qui." }
  },
  "header": { "link": { "title": "Una scorciatoia", "text": "La prossima volta incolla qui un <b>link a pipeline, MR, run o PR</b>: il report si crea senza aprire il progetto." } },
  "report": {
    "hotspots": { "title": "Da dove iniziare", "text": "Qui ci sono solo i job del <b>percorso critico</b>: velocizzare gli altri non accorcia la pipeline. Un clic porta al job." },
    "critical": { "title": "L'arancione è il percorso critico", "text": "Questi job determinano la durata della pipeline. <b>Un clic su uno stage</b> ricalcola il percorso fino alla sua fine." },
    "keys": { "title": "Zoom e tasti", "text": "<b>⌘/Ctrl + rotellina</b> per lo zoom, <b>[</b> e <b>]</b> per scorrere, <b>0</b> mostra tutta la pipeline. Il resto è sotto «?»." },
    "retries": { "title": "Retry", "text": "Il tratteggio rosso indica i tentativi falliti. L'etichetta <b>↻2 −8m29s</b> mostra quanti sono stati e il tempo perso." },
    "aggregate": { "title": "Come leggere l'aggregato", "text": "La barra è l'inizio e la durata <b>p50</b>, la linea che segue arriva al <b>p90</b>. L'etichetta a destra è la quota di esecuzioni con retry." },
    "detail": { "title": "Pannello dei dettagli", "text": "In alto c'è la <b>conclusione</b>: se accelerarlo accorcia la pipeline. Sotto, i collegamenti ↑/↓. Esc annulla la selezione." }
  }
}
```

`zh.json`:

```json
"tips": "提示",
"tipsShow": "显示长颈鹿提示",
"tipsReset": "重新显示"
```

```json
"onboarding": {
  "ok": "知道了",
  "never": "不再显示",
  "add-project": { "input": { "title": "这里填什么", "text": "可以填<b>仓库、流水线或 MR 的链接</b>、ssh 地址，或用右侧按钮选择克隆所在的文件夹。" } },
  "token": { "create": { "title": "需要令牌", "text": "链接会打开<b>已预设所需权限</b>的令牌创建页面。令牌保存在系统钥匙串中。" } },
  "project": {
    "screen": { "title": "单条流水线还是多条", "text": "点击流水线即可生成它的报告。<b>聚合</b>汇总该分支的 N 条流水线，并显示 p50 和 p90。" },
    "workflow": { "title": "Workflow", "text": "在 GitHub 上，运行记录和聚合<b>按单个 workflow</b> 生成——在这里选择。" }
  },
  "header": { "link": { "title": "更快的方式", "text": "下次可以直接在这里粘贴<b>流水线、MR、run 或 PR 的链接</b>，无需打开项目即可生成报告。" } },
  "report": {
    "hotspots": { "title": "从哪里入手", "text": "这里只列出<b>关键路径</b>上的 job：加速其他 job 不会缩短流水线。点击可跳转到该 job。" },
    "critical": { "title": "橙色是关键路径", "text": "这些 job 决定流水线的时长。<b>点击 stage</b> 会重新计算到该 stage 结束为止的路径。" },
    "keys": { "title": "缩放和快捷键", "text": "<b>⌘/Ctrl + 滚轮</b>缩放，<b>[</b> 和 <b>]</b> 平移，<b>0</b> 显示整条流水线。其余见“?”。" },
    "retries": { "title": "重试", "text": "红色斜线表示失败的尝试。标签 <b>↻2 −8m29s</b> 表示失败次数和因此损失的时间。" },
    "aggregate": { "title": "如何阅读聚合", "text": "色条表示开始时间和时长的 <b>p50</b>，其后的线延伸到 <b>p90</b>。右侧标签表示含重试的运行占比。" },
    "detail": { "title": "详情面板", "text": "顶部是<b>结论</b>：加速它能否缩短流水线。下方是 ↑/↓ 关联。按 Esc 取消选择。" }
  }
}
```

`ja.json`:

```json
"tips": "ヒント",
"tipsShow": "キリンのヒントを表示",
"tipsReset": "もう一度表示"
```

```json
"onboarding": {
  "ok": "了解",
  "never": "今後表示しない",
  "add-project": { "input": { "title": "ここに入力するもの", "text": "<b>リポジトリ、パイプライン、MR のリンク</b>、ssh アドレス、または右のボタンでクローンのフォルダーを選べます。" } },
  "token": { "create": { "title": "トークンが必要です", "text": "リンクから<b>必要な権限が設定済み</b>のトークン作成ページが開きます。トークンはシステムのキーチェーンに保存されます。" } },
  "project": {
    "screen": { "title": "1 本か、まとめてか", "text": "パイプラインをクリックするとそのレポートを作成します。<b>集計</b>はブランチの N 本をまとめて p50 と p90 を表示します。" },
    "workflow": { "title": "Workflow", "text": "GitHub では実行履歴と集計は<b>1 つの workflow ごと</b>に作成されます。ここで選択してください。" }
  },
  "header": { "link": { "title": "近道", "text": "次回はここに<b>パイプライン、MR、run、PR のリンク</b>を貼り付ければ、プロジェクトを開かずにレポートを作成できます。" } },
  "report": {
    "hotspots": { "title": "どこから始めるか", "text": "ここには<b>クリティカルパス</b>上の job だけが並びます。他を速くしてもパイプラインは短くなりません。クリックで job へ移動します。" },
    "critical": { "title": "オレンジはクリティカルパス", "text": "これらの job がパイプラインの長さを決めます。<b>stage をクリック</b>すると、その終わりまでのパスを再計算します。" },
    "keys": { "title": "ズームとキー", "text": "<b>⌘/Ctrl + ホイール</b>でズーム、<b>[</b> と <b>]</b> で移動、<b>0</b> でパイプライン全体を表示。その他は「?」にあります。" },
    "retries": { "title": "リトライ", "text": "赤い斜線は失敗した試行です。<b>↻2 −8m29s</b> は試行回数と失われた時間を示します。" },
    "aggregate": { "title": "集計の読み方", "text": "バーは開始と所要時間の <b>p50</b>、その後の線は <b>p90</b> までを示します。右のラベルはリトライを含む実行の割合です。" },
    "detail": { "title": "詳細パネル", "text": "上部に<b>結論</b>があります：高速化でパイプラインが短くなるかどうか。その下に ↑/↓ の関係。Esc で選択を解除します。" }
  }
}
```

Run: `npm run check:locales`
Expected: PASS.

- [ ] **Step 2: `TipBubble.tsx`**

```tsx
import { Trans } from 'react-i18next'
import { useTranslation } from '../../../shared/i18n'
import { Giraffe } from '../../../shared/ui/giraffe'
import { closeTip, disableTips } from '../model/store'
import type { TipId } from '../model/tips'

const ACCENT = { b: <b className="font-semibold text-crit-fg" /> }
const action = 'cursor-pointer rounded-sm focus-ring hover:underline'

/** Содержимое подсказки: жираф слева, заголовок, текст с выделениями, «Понятно» и «Больше не показывать». */
export function TipBubble({ id }: { id: TipId }) {
  const { t } = useTranslation()
  return (
    <div className="flex gap-3">
      <Giraffe size={56} />
      <div className="flex min-w-0 flex-col gap-1">
        <p className="text-[13px] font-semibold">{t(`onboarding.${id}.title`)}</p>
        <p className="text-[12.5px] leading-[1.4]">
          <Trans i18nKey={`onboarding.${id}.text`} components={ACCENT} />
        </p>
        <div className="mt-1.5 flex gap-3 text-xs">
          <button type="button" className={`${action} font-semibold text-crit-fg`} onClick={closeTip}>
            {t('onboarding.ok')}
          </button>
          <button type="button" className={`${action} text-muted-foreground`} onClick={disableTips}>
            {t('onboarding.never')}
          </button>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 3: `Tip.tsx`**

```tsx
import { useEffect, type ReactElement } from 'react'
import { Popover, PopoverAnchor, PopoverContent } from '../../../shared/ui/popover'
import { addReady, closeTip, removeReady, useTipOpen } from '../model/store'
import type { TipId } from '../model/tips'
import { TipBubble } from './TipBubble'

type Props = {
  id: TipId
  /** событие наступило; `false` — ребёнок без обёртки */
  when?: boolean
  /** один элемент, передающий ref и пропсы в DOM */
  children: ReactElement
}

/**
 * Подсказка у элемента. Поповер — в дереве якоря, а не в корне: внутри модального Dialog поповер из корня не получает кликов и фокуса.
 * Фокус не забирает, клик мимо не закрывает; Esc закрывает только подсказку (`stopPropagation` до `useReportKeys` и корневого обработчика).
 */
export function Tip({ id, when = true, children }: Props) {
  const open = useTipOpen(id)
  useEffect(() => {
    if (!when) return
    addReady(id)
    return () => removeReady(id)
  }, [id, when])
  if (!when) return children
  return (
    <Popover open={open} onOpenChange={(next) => !next && closeTip()}>
      <PopoverAnchor asChild data-tip-active={open ? '' : undefined}>
        {children}
      </PopoverAnchor>
      <PopoverContent
        side="bottom"
        align="start"
        collisionPadding={8}
        className="w-[300px] border-mascot-border bg-mascot-bg p-3 data-[state=open]:animate-tip-in motion-reduce:animate-none"
        onOpenAutoFocus={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
        onEscapeKeyDown={(e) => e.stopPropagation()}
      >
        <TipBubble id={id} />
      </PopoverContent>
    </Popover>
  )
}
```

- [ ] **Step 4: `OnboardingSync.tsx`, `useOnboardingVisit.ts`, `TipsSettings.tsx`, `index.ts`**

`OnboardingSync.tsx`:

```tsx
import { useEffect } from 'react'
import { useSettings, useUpdateSettings } from '../../../entities/settings'
import { hydrate, setPersist } from '../model/store'

/** Настройки → стор подсказок и запись просмотренных обратно. Только в форме: в сохранённом отчёте его нет, и подсказки выключены. */
export function OnboardingSync() {
  const { data } = useSettings()
  const update = useUpdateSettings()
  useEffect(() => setPersist((patch) => void update(patch)), [update])
  useEffect(() => {
    if (data) hydrate(data.tips, data.seenTips)
  }, [data])
  return null
}
```

`useOnboardingVisit.ts`:

```ts
import { useEffect } from 'react'
import { startVisit } from './store'

/** Экран смонтирован — новый визит: можно одну `enter`-подсказку. Эффект родителя идёт после эффектов детей-якорей, поэтому выбор — уже по готовым. */
export function useOnboardingVisit() {
  useEffect(startVisit, [])
}
```

`TipsSettings.tsx`:

```tsx
import { useId } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { useSettings, useUpdateSettings } from '../../../entities/settings'

/** «Показывать подсказки» и «Показать заново» (спека онбординга, § 4). */
export function TipsSettings() {
  const { t } = useTranslation()
  const id = useId()
  const { data } = useSettings()
  const update = useUpdateSettings()
  return (
    <div className="flex flex-wrap items-center gap-3">
      <label htmlFor={id} className="flex items-center gap-2 text-sm">
        <input id={id} type="checkbox" className="focus-ring" checked={data?.tips ?? true} onChange={(e) => void update({ tips: e.target.checked })} />
        {t('form.settings.tipsShow')}
      </label>
      <Button type="button" variant="outline" size="sm" onClick={() => void update({ tips: true, seenTips: [] })}>
        {t('form.settings.tipsReset')}
      </Button>
    </div>
  )
}
```

`index.ts`:

```ts
export { markLeftReport, useLeftReport } from './model/store'
export type { TipId } from './model/tips'
export { useOnboardingVisit } from './model/useOnboardingVisit'
export { OnboardingSync } from './ui/OnboardingSync'
export { Tip } from './ui/Tip'
export { TipsSettings } from './ui/TipsSettings'
```

- [ ] **Step 5: Подключить синхронизацию и блок настроек**

`app/src/app/form/main.tsx` — импорт `import { OnboardingSync } from '../../features/onboarding'`; в `render`:

```tsx
      <QueryClientProvider client={queryClient}>
        <OnboardingSync />
        <RouterProvider router={router} />
      </QueryClientProvider>
```

`SettingsPage.tsx` — импорт `import { TipsSettings } from '../../../features/onboarding'`; между секциями темы и языка:

```tsx
      <section className="flex flex-col gap-2">
        <h2 className={labelClasses}>{t('form.settings.tips')}</h2>
        <TipsSettings />
      </section>
```

- [ ] **Step 6: Проверка**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales && npm run test:onboarding`
Expected: PASS.

Run: `ls -l dist-report` на `main` и после `npm run build` в ветке.
Expected: `npm run build` проходит (собирает и шаблон отчёта, `vite.report.config.ts`); HTML отчёта вырос не больше чем на ~15 КБ. Больше — значит в отчёт попали `OnboardingSync`/react-query через `index.ts` слайса: проверить, что `radix-ui`/`zustand`/react-query не тянут побочных эффектов (`sideEffects`), и записать вывод в отчёт задачи.

Run: `npm run dev` → `/settings`: блок «Подсказки» между темой и языком; переключатель и кнопка меняют настройки (в моке — видно при повторном открытии страницы).

- [ ] **Step 7: Commit**

```bash
git add app/src/features/onboarding app/src/app/form/main.tsx app/src/pages/settings app/src/shared/i18n/locales
git commit -m "feat: подсказка с жирафом у элемента, синхронизация и настройки подсказок"
```

---

### Task 5: Подсказки приложения — диалог, токен, проект, шапка

**Files:**
- Modify: `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx:59,71`
- Modify: `app/src/widgets/app-header/ui/AppHeader.tsx:13-15`
- Modify: `app/src/widgets/aggregate-block/ui/AggregateBlock.tsx` (обёртка кнопки из задачи 2)
- Modify: `app/src/pages/project/ui/ProjectPage.tsx:13-36`
- Modify: `app/src/app/form/report-route.tsx:21-25`

**Interfaces:**
- Consumes: `Tip`, `useOnboardingVisit`, `markLeftReport`, `useLeftReport` из `features/onboarding` (задача 4).

- [ ] **Step 1: `add-project.input` и `token.create` в диалоге**

`AddProjectDialog.tsx` — импорт `import { Tip } from '../../../features/onboarding'`. Строку 59 (`<Input id={id} autoFocus … />`) обернуть:

```tsx
          <Tip id="add-project.input">
            <Input id={id} autoFocus translate="no" placeholder={t('form.addProject.placeholder')} aria-invalid={showError} aria-describedby={showError ? `${id}-error` : undefined} value={input} onChange={(e) => setInput(e.target.value)} />
          </Tip>
```

Строку 71:

```tsx
      {needsToken && (
        <Tip id="token.create">
          <div>
            <TokenBlock host={needsToken} onSaved={() => add(lastInput ?? input)} />
          </div>
        </Tip>
      )}
```

- [ ] **Step 2: `header.link` и `token.create` в шапке**

`AppHeader.tsx` — импорт `import { Tip, useLeftReport } from '../../../features/onboarding'`; в теле `const leftReport = useLeftReport()`; блок 13-15:

```tsx
      <Tip id="header.link" when={leftReport}>
        <div className="min-w-0 flex-1">
          <BuildByLink
            tokenBlock={(host, retry) => (
              <Tip id="token.create">
                <div>
                  <TokenBlock host={host} onSaved={retry} />
                </div>
              </Tip>
            )}
          />
        </div>
      </Tip>
```

`report-route.tsx` — импорт `import { markLeftReport } from '../../features/onboarding'`; эффект 21-25:

```tsx
  useEffect(() => {
    if (!valid) return
    void setCurrentReport(reportId)
    return () => {
      void setCurrentReport(null)
      // уход с отчёта — событие для подсказки поля ссылки в шапке
      markLeftReport()
    }
  }, [reportId, valid])
```

- [ ] **Step 3: `project.screen` и `project.workflow`, визит экрана проекта**

`AggregateBlock.tsx` — импорт `import { Tip } from '../../../features/onboarding'`; блок из задачи 2:

```tsx
      <div className="flex items-end gap-3">
        <Tip id="project.screen">
          <div className="inline-flex">
            <BuildAggregateButton busy={busy} progressLabel={progressLabel} onClick={start} />
          </div>
        </Tip>
        {busy && <Giraffe pose="loading" size={72} progress={progress ?? undefined} />}
      </div>
```

`ProjectPage.tsx` — импорт `import { Tip, useOnboardingVisit } from '../../../features/onboarding'`; первой строкой тела `useOnboardingVisit()`; блок 34-36:

```tsx
      {workflows.length > 0 && current.workflow && (
        <Tip id="project.workflow">
          <div>
            <WorkflowSelect workflows={workflows} value={current.workflow} onChange={(workflow) => void openProject({ ...current, workflow, name, replace: true })} />
          </div>
        </Tip>
      )}
```

- [ ] **Step 4: Проверка**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales`
Expected: PASS.

Run: `npm run dev`. В моке `seenTips: []`, проекты есть:
1. Экран проекта → подсказка `project.screen` у кнопки агрегата, кнопка в оранжевой обводке.
2. «Понятно» → закрылась; переход на другой проект в сайдбаре → подсказки нет (`project.workflow` только у GitHub-проекта; у моковых GitLab-проектов workflow нет).
3. ⌘⇧N → диалог, подсказка `add-project.input` у поля; ввод не прерывается, фокус в поле.
4. Esc → закрылась только подсказка, диалог открыт; второй Esc закрывает диалог.
5. Отчёт из строки пайплайна, «← Назад» → подсказка `header.link` у поля ссылки.

- [ ] **Step 5: Commit**

```bash
git add app/src/widgets app/src/pages/project app/src/app/form/report-route.tsx
git commit -m "feat: подсказки онбординга в диалоге, шапке и на экране проекта"
```

---

### Task 6: Подсказки отчёта

**Files:**
- Modify: `app/src/pages/report/ui/ReportPage.tsx` (визит)
- Modify: `app/src/widgets/hotspots/ui/Hotspots.tsx:35-49`
- Modify: `app/src/widgets/report-header/ui/ReportHeader.tsx:93`
- Modify: `app/src/widgets/detail-panel/ui/DetailPanel.tsx:26`
- Modify: `app/src/widgets/waterfall/ui/Waterfall.tsx` (первые строки для подсказок)
- Modify: `app/src/widgets/waterfall/ui/Row.tsx` (три флага, обёртки)

**Interfaces:**
- Consumes: `Tip`, `useOnboardingVisit` из `features/onboarding`; `isLeaf` из `entities/report`; `isAggNode` из `shared/api`.
- `Row` получает примитивные пропсы `tipCritical: boolean`, `tipRetries: boolean`, `tipPill: boolean` — `Row` под `memo`, объекты ломали бы его.

- [ ] **Step 1: Визит и `report.hotspots`**

`ReportPage.tsx` — импорт `import { useOnboardingVisit } from '../../../features/onboarding'`; в `ReportPage` первой строкой:

```tsx
  useOnboardingVisit()
```

`Hotspots.tsx` — импорт `import { Tip } from '../../../features/onboarding'`; возвращаемую карточку обернуть (`Card` передаёт пропсы и ref в `<div>`), содержимое карточки без изменений:

```tsx
  return (
    <Tip id="report.hotspots">
      <Card className="mt-px mb-2.5 gap-0 overflow-hidden p-0 pb-1.5">
        <h2 className="m-0 flex items-center gap-2 px-3 pt-2 pb-0.5 text-[13px] font-semibold">
          <span aria-hidden className="size-2 rounded-full bg-crit" />
          {t('report.hotspots.title')}
        </h2>
        {tree.hotspots.map((h) => (
          <button key={`${h.id}:${h.kind}`} type="button" className="flex w-full cursor-pointer items-baseline gap-2 px-3 py-[3px] text-left hover:bg-accent" onClick={() => reveal(h.id)}>
            <span className="min-w-[62px] font-semibold text-crit-fg">−{duration(h.saving)}</span>
            <span>
              <span translate="no">{h.name}</span> <span className="text-muted-foreground">— <Reason hotspot={h} /></span>
            </span>
          </button>
        ))}
      </Card>
    </Tip>
  )
```

- [ ] **Step 2: `report.keys` и `report.detail`**

`ReportHeader.tsx` — импорт `import { Tip } from '../../../features/onboarding'`; строку 93:

```tsx
        <Tip id="report.keys">
          <span className="inline-flex">
            <KeysHelp />
          </span>
        </Tip>
```

`DetailPanel.tsx` — импорт `import { Tip } from '../../../features/onboarding'`; строку 26 (панель монтируется на первом выборе — это и есть событие):

```tsx
      <Tip id="report.detail">
        <div>
          <PanelHead node={node} />
        </div>
      </Tip>
```

- [ ] **Step 3: Первые строки для подсказок в `Waterfall.tsx`**

Импорты: `import { cards, isLeaf, useReportView } from '../../../entities/report'` (добавить `isLeaf`), `import { isAggNode, type ReportNode } from '../../../shared/api'` (добавить `isAggNode`).

После `const blocks = useMemo(…)`:

```tsx
  // по одной строке на подсказку онбординга: первая подходящая в порядке отрисовки
  const tipRows = useMemo(() => {
    const agg = isAggNode(tree.nodes[tree.root])
    let critical: string | undefined
    let retries: string | undefined
    let pill: string | undefined
    for (const { rows: entries } of blocks) {
      for (const { node } of entries) {
        if (critical === undefined && isLeaf(node) && crit.ids.has(node.id)) critical = node.id
        if (retries === undefined && !isAggNode(node) && node.attempts.length > 0) retries = node.id
        if (pill === undefined && agg && node.stability?.present) pill = node.id
      }
    }
    return { critical, retries, pill }
  }, [blocks, tree, crit])
```

Если `!isAggNode(node)` не сужает тип до узла с `attempts` (проверить `shared/api/schema/SingleNode.ts` и guard `isAggNode`), заменить условие на `'attempts' in node && node.attempts.length > 0`.

В `<Row … />` после `critical={crit.ids.has(node.id)}`:

```tsx
                tipCritical={tipRows.critical === node.id}
                tipRetries={tipRows.retries === node.id}
                tipPill={tipRows.pill === node.id}
```

- [ ] **Step 4: Обёртки в `Row.tsx`**

Импорт `import { Tip } from '../../../features/onboarding'`. В `RowProps` после `critical: boolean`:

```tsx
  /** якорь подсказки онбординга: первая строка критического пути, с ретраями, с меткой стабильности */
  tipCritical: boolean
  tipRetries: boolean
  tipPill: boolean
```

Добавить `tipCritical, tipRetries, tipPill` в деструктуризацию пропсов. Корневой `<div {...rowAttrs} …>…</div>` обернуть в `<Tip id="report.retries" when={tipRetries}>…</Tip>` (ref-колбэк строки Slot склеивает с ref якоря). Дорожку и метку заменить:

```tsx
      <Tip id="report.critical" when={tipCritical}>
        <div {...laneAttrs} className="relative cursor-grab overflow-hidden">
          <div {...trackAttrs} className="absolute inset-y-0 right-6 left-3">
            <NodeLane node={node} view={view} critical={critical} excess={excess} holderName={holderName} gapMs={gapMs} />
          </div>
          {dep && <DepTag dir={dep} name={depName} />}
        </div>
      </Tip>
      <Tip id="report.aggregate" when={tipPill}>
        <div className="flex items-center justify-end pr-2.5">
          <StabilityPill stability={node.stability} />
        </div>
      </Tip>
```

`<Tip when={false}>` возвращает ребёнка без обёртки — у сотен строк нет лишних поповеров.

- [ ] **Step 5: Проверка**

Run: `npm run typecheck && npm run lint:fsd && npm run build`
Expected: PASS.

Run: `npm run dev` (мок отдаёт агрегатный отчёт), «Показать заново» в настройках:
1. Открыть отчёт → `report.hotspots` у карточки «Куда направить силы».
2. «Понятно», клик по джобе → `report.detail` у шапки панели. Esc → закрылась только подсказка, панель открыта; второй Esc снимает выбор.
3. «← Назад», открыть отчёт снова → `report.critical` у первой оранжевой полоски; ещё раз → `report.keys` у `?`; ещё раз → `report.aggregate` у метки стабильности (у агрегата из мока может не быть ретраев — `report.retries` проверяется в Tauri на реальном пайплайне, задача 7).

- [ ] **Step 6: Commit**

```bash
git add app/src/pages/report app/src/widgets
git commit -m "feat: подсказки онбординга в отчёте"
```

---

### Task 7: Документы и приёмка

**Files:**
- Modify: `docs/specs/2026-10-08-onboarding-design.md` (новый § 7)
- Modify: `docs/specs/2026-10-06-first-run-ux-design.md` (§ 1, «Не делается»)
- Modify: `docs/specs/2026-10-06-native-redesign-design.md` (§ 1, правила uupm)
- Modify: `docs/specs/2026-10-03-tauri-rewrite-acceptance.md` (новый раздел чек-листа)
- Modify: `README.md` (раздел «Приложение»)

- [ ] **Step 1: Спека онбординга**

В конец `docs/specs/2026-10-08-onboarding-design.md`:

```markdown
## 7. Отступления при реализации

- `reportExpired` показывается системным окном — жираф `oops` только под полем «Добавить проект» и в блоке токена.
- `token.create` привязана ко всему блоку токена: `features/manage-token` не может импортировать `features/onboarding`.
- Жираф `loading` — у кнопки агрегата и на экране загрузки отчёта; в кнопке «Построить» шапки и в строке пайплайна — прежний текст прогресса.
- Esc: подсказка гасит распространение в `onEscapeKeyDown`; корневой обработчик и `useReportKeys` не меняются.
- Визит отмечают `ProjectPage` и `ReportPage`.
- Активная подсказка уходит при размонтировании якоря через микротаск (StrictMode).
```

- [ ] **Step 2: Ссылки в базовых спеках**

`2026-10-06-first-run-ux-design.md`, § 1, строка «Не делается»: `онбординг-туры` → `онбординг-туры (подсказки по событиям — см. 2026-10-08-onboarding-design.md)`.

`2026-10-06-native-redesign-design.md`, § 1, абзац правил uupm: `(анимаций не добавляем)` → `(анимаций не добавляем, кроме подсказок и жирафа — см. 2026-10-08-onboarding-design.md)`.

- [ ] **Step 3: README**

В разделе «Приложение» (`README.md`, список возможностей, последним пунктом):

```markdown
- подсказки с жирафом появляются при первой встрече с элементом — диалогом, блоком токена, отчётом, панелью деталей; выключить и показать заново — в настройках, блок «Подсказки».
```

Предыдущий пункт списка, если кончался точкой, — на `;`.

- [ ] **Step 4: Чек-лист приёмки**

В конец `docs/specs/2026-10-03-tauri-rewrite-acceptance.md` — раздел `## Онбординг (2026-10-08)`, в нём пункты из спеки онбординга, § 5 («Чек-лист приёмки»), по одному на строку с `- [ ]`, плюс:

```markdown
- [ ] `report.retries` у первой строки с ретраями на реальном пайплайне с ретраями
- [ ] Перезапуск приложения: показанные подсказки не возвращаются
- [ ] Сохранённый HTML-отчёт открывается в браузере без подсказок и ошибок в консоли
```

- [ ] **Step 5: Приёмка в Tauri**

Run: `npm run tauri dev` с чистым каталогом данных (macOS: переименовать `~/Library/Application Support/dev.longpole.desktop`, после проверки вернуть имя). Пройти чек-лист из шага 4, тёмную тему и «Уменьшить движение» в системных настройках.

- [ ] **Step 6: Commit**

```bash
git add docs README.md
git commit -m "docs: онбординг — отступления, ссылки, README, чек-лист приёмки"
```
