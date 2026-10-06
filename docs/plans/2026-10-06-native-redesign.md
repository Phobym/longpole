# Редизайн «Нативный desktop» — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Форма и отчёт выглядят как системное окно macOS/Windows (source-list сайдбар, тулбар, push-кнопки, inset-grouped карточки, инспектор, системный синий), поведение и тексты не меняются.

**Architecture:** Меняются только токены в `shared/ui/theme.css`, примитивы `shared/ui/*` и классы Tailwind в перечисленных компонентах; логика не трогается. Прототип (`app/src/app/prototype/`, переключатель `?variant=`) уезжает в ветку `proto/redesign-variants`, его мок Tauri-моста остаётся dev-инструментом в `app/src/app/dev/mock.ts`. Каждое правило прототипа для `native` превращается в утилиту в своём компоненте — прототипные селекторы по классам не переносятся.

**Tech Stack:** React 19, Tailwind v4 (`@theme static`, `@custom-variant dark`), Radix (`radix-ui`), lucide-react, TanStack Router, Vite 8, `vite-plugin-singlefile` для отчёта.

Спека — `docs/specs/2026-10-06-native-redesign-design.md` (далее «спека, § N»). Правила репозитория: `docs/agents/code-smells.md` (проход по диффу перед коммитом), комментарии и тексты — по-русски. Автотестов фронтенда нет (спека, § 1): проверка каждой задачи — `npm run typecheck`, `npm run lint:fsd` и скриншот в браузере на моке (`npm run dev`, `http://localhost:5173/`), сравнение с эталоном прототипа.

## Global Constraints

- Паритет: экраны, поведение, клавиши и тексты не меняются (спека, § 1); i18n-ключи не добавляются и не удаляются.
- Шрифты только системные, никаких `@font-face` и внешних запросов: отчёт — один HTML с CSP на хешах (спека, § 1).
- Тёмная тема — класс `dark` на `<html>` (`shared/lib/theme.ts`); утилиты `dark:` работают только после `@custom-variant dark` из задачи 3.
- Цвета — только через токены `--color-*`; «сырые» hex в компонентах недопустимы, кроме градиентов кнопок из спеки, § 3.
- Контраст текста ≥ 4,5:1 в обеих темах (спека, § 7).
- Все команды — из `app/`: `npm run typecheck`, `npm run lint:fsd`, `npm run build`, `npm run dev`.
- Коммиты — на ветке `Phobym/redisign`, сообщения по образцу истории: `feat(web): …`, `style(web): …`, `chore: …`.

---

### Task 1: Прототип — в ветку `proto/redesign-variants`, эталонный worktree

**Files:**
- Commit (в ветку `proto/redesign-variants`): `app/src/app/prototype/switcher.tsx`, `app/src/app/prototype/variants.css`, `app/src/app/prototype/mock.ts`, `app/src/app/form/main.tsx`, `app/src/app/report/main.tsx`

**Interfaces:**
- Produces: ветка `proto/redesign-variants` с прототипом; worktree `../redisign-proto` на порту 5174 как эталон `?variant=native` для всех следующих задач.

- [ ] **Step 1: Убедиться, что в рабочем дереве только прототип и документы**

Run: `git status --short`
Expected:
```
 M app/src/app/form/main.tsx
 M app/src/app/report/main.tsx
?? app/src/app/prototype/
?? docs/specs/2026-10-06-native-redesign-design.md
?? docs/plans/2026-10-06-native-redesign.md
```
(`.playwright-mcp/` может присутствовать — это логи скриншотов, в коммиты не идёт; удалить `rm -rf .playwright-mcp`.)

- [ ] **Step 2: Закоммитить прототип в отдельную ветку**

```bash
git checkout -b proto/redesign-variants
git add app/src/app/prototype app/src/app/form/main.tsx app/src/app/report/main.tsx
git commit -m "proto(web): четыре варианта визуального языка по ?variant= и мок моста Tauri для браузера"
git checkout Phobym/redisign
```

Expected: `git status --short` показывает только `?? docs/specs/…` и `?? docs/plans/…`; каталога `app/src/app/prototype/` в дереве нет.

- [ ] **Step 3: Поднять эталонный worktree прототипа на 5174**

```bash
git worktree add ../redisign-proto proto/redesign-variants
cd ../redisign-proto/app && npm ci && (npx vite --port 5174 > /dev/null 2>&1 &)
cd -
```

Expected: `curl -s -o /dev/null -w "%{http_code}" http://localhost:5174/` → `200`. Эталоны: `http://localhost:5174/?variant=native` (форма) и `http://localhost:5174/report.html?fixture=aggregate&variant=native` (отчёт). Тёмная тема — кнопка ☾ в плавающей панели.

- [ ] **Step 4: Закоммитить спеку и план**

```bash
git add docs/specs/2026-10-06-native-redesign-design.md docs/plans/2026-10-06-native-redesign.md
git commit -m "docs: спека и план редизайна «Нативный desktop»"
```

- [ ] **Step 5: Задача в GitHub Issues**

```bash
gh issue create --title "Редизайн: нативный desktop" --body "$(cat <<'EOF'
Визуальный язык формы и отчёта — «как системное окно». Спека: `docs/specs/2026-10-06-native-redesign-design.md`, план: `docs/plans/2026-10-06-native-redesign.md`.

Прототип четырёх вариантов (current / swiss / mono / native) — ветка `proto/redesign-variants`, переключатель `?variant=`. Выбран `native`.
EOF
)"
```

Expected: ссылка на issue. Если `gh` не достучался до GitHub — повторить позже, остальные задачи не блокируются.

---

### Task 2: Мок моста Tauri — dev-инструмент в `app/dev`

**Files:**
- Create: `app/src/app/dev/mock.ts` (из `app/src/app/prototype/mock.ts` ветки `proto/redesign-variants`, без изменений содержимого кроме шапки комментария)
- Modify: `app/src/app/form/main.tsx:19-21`

**Interfaces:**
- Produces: `npm run dev` в браузере без Tauri показывает форму с данными (проекты, недавние, пайплайны, настройки, отчёт по сборке). Используется для проверки всех следующих задач.

- [ ] **Step 1: Перенести мок из ветки прототипа**

```bash
mkdir -p app/src/app/dev
git show proto/redesign-variants:app/src/app/prototype/mock.ts > app/src/app/dev/mock.ts
```

Заменить шапку файла (первые строки до `import`) на:

```ts
/**
 * Мост Tauri в обычном браузере (`npm run dev` без `tauri dev`): `window.__TAURI_INTERNALS__` с данными в памяти,
 * чтобы форму можно было смотреть и верстать без Rust. Подключается только в dev и только когда настоящего моста нет;
 * в сборку не попадает (`import.meta.env.DEV`).
 */
```

- [ ] **Step 2: Подключить мок в `app/src/app/form/main.tsx`**

Было:
```ts
async function main() {
  const root = document.getElementById('root')
```
Стало:
```ts
async function main() {
  // в браузере без Tauri — мок моста, иначе `get_settings` падает на `invoke`
  if (import.meta.env.DEV) await import('../dev/mock')
  const root = document.getElementById('root')
```

- [ ] **Step 3: Проверить**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run build && grep -c "mock: нет команды" dist/assets/*.js; cd -`
Expected: `No errors found`, `No problems found!`, сборка зелёная, `grep` печатает `0` для каждого файла (мока в бандле нет).

Run: `cd app && (npx vite --port 5173 > /dev/null 2>&1 &); cd -` и открыть `http://localhost:5173/`.
Expected: сайдбар с тремя проектами и тремя записями «Недавние», экран проекта `group / project` с восемью пайплайнами, без красных ошибок.

- [ ] **Step 4: Commit**

```bash
git add app/src/app/dev/mock.ts app/src/app/form/main.tsx
git commit -m "chore(web): мок моста Tauri для dev в браузере"
```

---

### Task 3: Токены, шрифт, вариант `dark`

**Files:**
- Modify: `app/src/shared/ui/theme.css` (целиком)
- Modify: `app/report.html:5-6`

**Interfaces:**
- Produces: токены из спеки, § 2, включая новые `--color-chrome`, `--color-sidebar`, `--color-link` (утилиты `bg-chrome`, `bg-sidebar`, `text-link`); утилита `focus-ring` (ореол + внутренняя линия, заменяет `outline-hidden focus-visible:ring-…`); вариант `dark:`; `--radius: 8px`. Все следующие задачи опираются на эти имена.

- [ ] **Step 1: Переписать `app/src/shared/ui/theme.css`**

```css
@import 'tailwindcss';

/* Тёмная тема — класс `dark` на `<html>` (`shared/lib/theme.ts`); утилиты `dark:` смотрят на него, а не на prefers-color-scheme. */
@custom-variant dark (&:where(.dark, .dark *));

/*
 * Палитра «нативный desktop» (спека docs/specs/2026-10-06-native-redesign-design.md, § 2): системный синий — единственный акцент,
 * `chrome` — фон тулбара, контента формы, инспектора и диалога, `sidebar` — фон сайдбара. Контраст текста ≥ 4,5:1 в обеих темах.
 * `static`: переменные нужны и без утилит (inline-стили, var(--color-*)).
 */
@theme static {
  --color-background: #ffffff;
  --color-foreground: #1d1d1f;
  --color-chrome: #f5f5f7;
  --color-sidebar: #ececf0;
  --color-card: #ffffff;
  --color-card-foreground: #1d1d1f;
  --color-popover: #ffffff;
  --color-popover-foreground: #1d1d1f;
  --color-primary: #0a6fe6;
  --color-primary-foreground: #ffffff;
  --color-secondary: #f2f2f7;
  --color-secondary-foreground: #1d1d1f;
  --color-muted: #f5f5f7;
  --color-muted-foreground: #636368;
  --color-accent: #eeeef2;
  --color-accent-foreground: #1d1d1f;
  --color-destructive: #d0281f;
  --color-border: #d8d8dc;
  --color-input: #c9c9cf;
  --color-ring: #0a6fe6;
  --color-selected: #dbe8ff;
  /* текстовые ссылки: primary как заливка с белым текстом и как цвет текста не проходят 4,5:1 одновременно */
  --color-link: #0b63cc;

  /* домен: статусы, критический путь, повторы и превышения, связи */
  --color-ok: #34c759;
  --color-fail: #ff3b30;
  --color-warn: #ffcc00;
  --color-run: #0a6fe6;
  --color-idle: #c7c7cc;
  --color-queued: #e5e5ea;
  --color-thin: #d1d1d6;
  --color-wait: #e5e5ea;
  --color-crit: #ff9500;
  --color-crit-fg: #a35500;
  --color-crit-soft: #ffd9a8;
  --color-crit-bg: #fff7eb;
  --color-over-a: var(--color-crit);
  --color-over-b: #ffe2bf;
  --color-retry-a: var(--color-fail);
  --color-retry-b: #ffd0cd;
  --color-retry-fg: #bf231a;
  --color-dep-up: #0a6fe6;
  --color-dep-down: #187232;
  --color-pill-ok-bg: #e3f7e8;
  --color-pill-ok-fg: #187232;
  --color-pill-mid-bg: #fff5cc;
  --color-pill-mid-fg: #8a6400;
  --color-pill-bad-bg: #ffe3e1;
  --color-pill-bad-fg: #bf231a;
  --color-pill-run-bg: #e1eeff;
  --color-pill-run-fg: #0a5fd0;

  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);
}

:root {
  color-scheme: light;
  /* в px, а не rem: корневой размер шрифта 13px, md = 6px (контролы), lg = 8px, xl = 12px (диалог) */
  --radius: 8px;
}

:root.dark {
  color-scheme: dark;
  --color-background: #1e1e1e;
  --color-foreground: #f5f5f7;
  --color-chrome: #2b2b2d;
  --color-sidebar: #242426;
  --color-card: #262626;
  --color-card-foreground: #f5f5f7;
  --color-popover: #2c2c2e;
  --color-popover-foreground: #f5f5f7;
  --color-primary: #1f6fdc;
  --color-primary-foreground: #ffffff;
  --color-secondary: #2c2c2e;
  --color-secondary-foreground: #f5f5f7;
  --color-muted: #2a2a2a;
  --color-muted-foreground: #a1a1a6;
  --color-accent: #333336;
  --color-accent-foreground: #f5f5f7;
  --color-destructive: #ff6961;
  --color-border: #3a3a3c;
  --color-input: #48484a;
  /* ring светлее primary: сплошная линия фокуса должна давать ≥ 3:1 на chrome */
  --color-ring: #5aa2ff;
  --color-selected: #18304f;
  --color-link: #5aa2ff;
  --color-ok: #30d158;
  --color-fail: #ff453a;
  --color-warn: #ffd60a;
  --color-run: #1f6fdc;
  --color-idle: #48484a;
  --color-queued: #3a3a3c;
  --color-thin: #48484a;
  --color-wait: #3a3a3c;
  --color-crit: #ff9f0a;
  --color-crit-fg: #ffb84d;
  --color-crit-soft: #5a3d10;
  --color-crit-bg: #2a2416;
  --color-over-b: #4a3a1c;
  --color-retry-b: #4a2422;
  --color-retry-fg: #ff6961;
  --color-dep-up: #64d2ff;
  --color-dep-down: #30d158;
  --color-pill-ok-bg: #1d3a27;
  --color-pill-ok-fg: #4ade80;
  --color-pill-mid-bg: #3b3414;
  --color-pill-mid-fg: #ffd60a;
  --color-pill-bad-bg: #3f1f1d;
  --color-pill-bad-fg: #ff6961;
  --color-pill-run-bg: #1c2d4a;
  --color-pill-run-fg: #64d2ff;
}

/*
 * Фокус как в macOS: ореол 3px на 35% и сплошная внутренняя линия 1px цвета ring — она даёт ≥ 3:1 там, где ореол слишком бледный,
 * и не режется `overflow: hidden` контейнеров списков. Одна утилита вместо повторения четырёх классов в каждом контроле.
 */
@utility focus-ring {
  outline: none;
  &:focus-visible {
    box-shadow:
      inset 0 0 0 1px var(--color-ring),
      0 0 0 3px color-mix(in srgb, var(--color-ring) 35%, transparent);
  }
}

@layer base {
  * {
    @apply border-border outline-ring/50;
  }
  html {
    /* системный шрифт ОС; 13px — как в macOS; rem-утилиты Tailwind масштабируются от него, как в прототипе */
    font-family:
      -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Segoe UI Variable', 'Segoe UI', system-ui, Roboto, sans-serif;
    font-size: 13px;
    -webkit-font-smoothing: antialiased;
  }
  body {
    @apply bg-background text-foreground;
    font-variant-numeric: tabular-nums;
  }
  /* Tailwind v4 не ставит указатель кнопкам; пункты Radix Select и cmdk — role option */
  button:not(:disabled),
  a[href],
  summary,
  label:has(> input:not(:disabled)),
  [role='option'],
  [role='menuitem'] {
    cursor: pointer;
  }
}
```

- [ ] **Step 2: `theme-color` в `app/report.html`**

Было:
```html
    <meta name="theme-color" content="#f7f8fa" media="(prefers-color-scheme: light)" />
    <meta name="theme-color" content="#111318" media="(prefers-color-scheme: dark)" />
```
Стало:
```html
    <meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)" />
    <meta name="theme-color" content="#1e1e1e" media="(prefers-color-scheme: dark)" />
```

- [ ] **Step 3: Проверить**

Run: `cd app && npm run typecheck && npm run build; cd -`
Expected: зелёно. В браузере `http://localhost:5173/` — фон белый, кнопка «Построить» синяя (`#0a6fe6`), шрифт системный 13px; `http://localhost:5173/report.html?fixture=aggregate` — полоски success зелёные, критический путь оранжевый. Тёмная тема: в консоли `document.documentElement.classList.add('dark')` — фон `#1e1e1e`.

Контраст (любой калькулятор WCAG): `#636368` на `#ffffff` ≈ 6,0, на `#f5f5f7` ≈ 5,5, на сайдбаре `#ececf0` ≈ 5,1; `#ffffff` на `#0a6fe6` ≈ 4,7; `#0b63cc` на `#f5f5f7` ≈ 5,3; `#a35500` на `#fff7eb` ≈ 5,1; `#bf231a` на `#ffe3e1` ≈ 5,0; `#187232` на `#e3f7e8` ≈ 5,4; тёмная: `#a1a1a6` на `#262626` ≈ 5,8 и на `#18304f` ≈ 5,2; `#ffffff` на `#1f6fdc` ≈ 4,8; `#5aa2ff` на `#2b2b2d` ≈ 5,4. Все ≥ 4,5. Линия фокуса (`ring`): `#0a6fe6` на `#f5f5f7` ≈ 4,3, `#5aa2ff` на `#2b2b2d` ≈ 5,4 — ≥ 3:1 для нетекстового индикатора.

- [ ] **Step 4: Commit**

```bash
git add app/src/shared/ui/theme.css app/report.html
git commit -m "style(web): токены «нативный desktop», системный шрифт 13px, вариант dark по классу"
```

---

### Task 4: Примитивы `shared/ui`: Button, Input, Select, Segmented, Card, Badge, Dialog

**Files:**
- Modify: `app/src/shared/ui/button.tsx` (целиком)
- Modify: `app/src/shared/ui/input.tsx` (целиком)
- Modify: `app/src/shared/ui/select.tsx:10-27` (`SelectTrigger`)
- Create: `app/src/shared/ui/segmented.tsx`
- Modify: `app/src/shared/ui/card.tsx:4-6`
- Modify: `app/src/shared/ui/badge.tsx` (целиком)
- Modify: `app/src/shared/ui/dialog.tsx:14-26` (`DialogContent`)
- Modify: `app/src/shared/ui/paged-list.tsx:27`

**Interfaces:**
- Produces:
  - `Button` варианты `default | destructive | outline | secondary | ghost`, размеры `default (26px) | sm (22px) | icon (26×26)`; `buttonVariants` как раньше.
  - `inputClasses: string`, `labelClasses: string` из `input.tsx` — подписи полей формы (спека, § 4).
  - `Segmented<T extends string>({ label, value, options, onChange, size? })` — `role="group"`, пункты с `aria-pressed`; `options: { value: T; label: string; lang?: string }[]`; `size: 'sm' | 'xs'` (22px и 20px).
  - `Badge` варианты как раньше, новый вид по спеке, § 3.

- [ ] **Step 1: `app/src/shared/ui/button.tsx`**

```tsx
import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// push-кнопки macOS: лёгкий градиент и полупиксельная тень, высота 26px (спека, § 3)
export const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-2 rounded-md text-[13px] font-medium whitespace-nowrap transition-colors focus-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
  {
    variants: {
      variant: {
        default:
          'bg-linear-to-b from-[#3a97ff] to-primary text-primary-foreground shadow-[0_0.5px_1px_rgb(0_0_0/0.25),inset_0_0.5px_0_rgb(255_255_255/0.25)] hover:brightness-105',
        destructive: 'bg-destructive text-background hover:bg-destructive/90',
        outline:
          'border border-black/12 bg-linear-to-b from-card to-secondary shadow-[0_0.5px_1px_rgb(0_0_0/0.15)] hover:from-secondary dark:border-white/10 dark:from-[#5a5a5e] dark:to-[#4a4a4e] dark:hover:from-[#4a4a4e]',
        secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
        ghost: 'hover:bg-accent hover:text-accent-foreground',
      },
      size: {
        default: 'h-[26px] px-3',
        sm: 'h-[22px] gap-1.5 px-2.5 text-xs',
        icon: 'size-[26px]',
      },
    },
    defaultVariants: { variant: 'default', size: 'default' },
  },
)

export const Button = ({ className, variant, size, ...props }: ComponentProps<'button'> & VariantProps<typeof buttonVariants>) => (
  <button className={cn(buttonVariants({ variant, size }), className)} {...props} />
)
```

- [ ] **Step 2: `app/src/shared/ui/input.tsx`**

```tsx
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// поле macOS: 26px, внутренняя полупиксельная тень, синий ореол на фокусе (спека, § 3)
export const inputClasses =
  'h-[26px] w-full min-w-0 rounded-md border border-black/18 bg-card px-2 text-[13px] shadow-[inset_0_0.5px_1px_rgb(0_0_0/0.08)] focus-ring transition-colors placeholder:text-muted-foreground focus-visible:border-primary disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive dark:border-white/12 dark:bg-[#1a1a1a]'

/** Подпись поля формы: 11px капсом, приглушённая (спека, § 4). */
export const labelClasses = 'text-[11px] font-medium tracking-[.02em] text-muted-foreground uppercase'

export const Input = ({ className, ...props }: ComponentProps<'input'>) => (
  <input className={cn(inputClasses, className)} {...props} />
)
```

- [ ] **Step 3: `SelectTrigger` в `app/src/shared/ui/select.tsx`**

Заменить строку классов `SelectTrigger`:
```tsx
    className={cn(
      'flex h-[26px] w-full items-center justify-between gap-2 rounded-md border border-black/12 bg-linear-to-b from-card to-secondary px-2.5 text-[13px] whitespace-nowrap shadow-[0_0.5px_1px_rgb(0_0_0/0.15)] focus-ring disabled:pointer-events-none disabled:opacity-50 aria-invalid:border-destructive data-[placeholder]:text-muted-foreground dark:border-white/10 dark:from-[#5a5a5e] dark:to-[#4a4a4e]',
      className,
    )}
```

- [ ] **Step 4: Создать `app/src/shared/ui/segmented.tsx`**

```tsx
import { cn } from '../lib/cn'

type Option<T extends string> = { value: T; label: string; lang?: string }

type Props<T extends string> = {
  /** Доступное имя группы. */
  label: string
  value: T
  options: Option<T>[]
  onChange: (value: T) => void
  /** `sm` — 22px (форма), `xs` — 20px (шапка отчёта). */
  size?: 'sm' | 'xs'
}

/** Сегментированный переключатель как в macOS: подложка `secondary`, активный пункт приподнят тенью. */
export function Segmented<T extends string>({ label, value, options, onChange, size = 'sm' }: Props<T>) {
  return (
    <div role="group" aria-label={label} className="inline-flex rounded-md bg-secondary p-px">
      {options.map((option) => {
        const active = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            lang={option.lang}
            aria-pressed={active}
            onClick={() => onChange(option.value)}
            className={cn(
              'rounded-[5px] px-2.5 font-medium whitespace-nowrap focus-ring',
              size === 'sm' ? 'h-[22px] text-xs' : 'h-5 text-[11px]',
              active ? 'bg-card shadow-[0_0.5px_2px_rgb(0_0_0/0.2)]' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {option.label}
          </button>
        )
      })}
    </div>
  )
}
```

- [ ] **Step 5: `Card` в `app/src/shared/ui/card.tsx`**

Заменить `Card`:
```tsx
// inset-grouped карточка macOS: 10px, полупрозрачная hairline, едва заметная тень (спека, § 3)
export const Card = ({ className, ...props }: ComponentProps<'div'>) => (
  <div
    className={cn('flex flex-col gap-4 rounded-[10px] border border-black/10 bg-card px-[18px] py-4 text-card-foreground shadow-[0_1px_2px_rgb(0_0_0/0.04)] dark:border-white/8', className)}
    {...props}
  />
)
```

- [ ] **Step 6: `app/src/shared/ui/badge.tsx`**

```tsx
import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'
import { cn } from '../lib/cn'

// ok/run/mid/bad — метки статусов пайплайна и стабильности, скруглённые теги 4px;
// chip/chipCrit/chipBad — чипы шапки и панели отчёта: без рамки, на подложке, 6px (спека, § 3).
const badgeVariants = cva('inline-flex w-fit shrink-0 items-center justify-center gap-1 px-2 py-0.5 text-xs whitespace-nowrap', {
  variants: {
    variant: {
      secondary: 'rounded-md bg-secondary font-medium text-secondary-foreground',
      ok: 'rounded bg-pill-ok-bg font-medium text-pill-ok-fg',
      run: 'rounded bg-pill-run-bg font-medium text-pill-run-fg',
      mid: 'rounded bg-pill-mid-bg font-medium text-pill-mid-fg',
      bad: 'rounded bg-pill-bad-bg font-medium text-pill-bad-fg',
      chip: 'rounded-md bg-secondary text-muted-foreground [&_b]:font-semibold [&_b]:text-foreground',
      chipCrit: 'rounded-md bg-crit-bg text-crit-fg [&_b]:font-semibold [&_b]:text-crit-fg',
      chipBad: 'rounded-md bg-pill-bad-bg text-retry-fg [&_b]:font-semibold [&_b]:text-retry-fg',
    },
  },
  defaultVariants: { variant: 'secondary' },
})

export function Badge({ className, variant, ...props }: ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
```

- [ ] **Step 7: `DialogContent` в `app/src/shared/ui/dialog.tsx`**

Заменить строку классов `DialogPrimitive.Content`:
```tsx
      className={cn(
        'fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-full max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col gap-4 overflow-y-auto rounded-xl bg-chrome p-6 text-card-foreground shadow-[0_20px_60px_rgb(0_0_0/0.3),0_0_0_0.5px_rgb(0_0_0/0.2)] outline-hidden',
        className,
      )}
```

- [ ] **Step 8: Контейнер списка в `app/src/shared/ui/paged-list.tsx`**

Было: `<ul className="flex flex-col gap-1.5">{items.map(children)}</ul>`
Стало:
```tsx
      {/* рамка и zebra — у контейнера, строки без своих рамок (спека, § 4); пустой список не рисует пустую рамку */}
      <ul className="flex flex-col overflow-hidden rounded-lg border empty:hidden">{items.map(children)}</ul>
```

- [ ] **Step 9: Проверить**

Run: `cd app && npm run typecheck && npm run lint:fsd; cd -`
Expected: зелёно (`Segmented` пока никем не используется — это нормально).
В браузере `http://localhost:5173/`: кнопка «Построить» 26px с градиентом, поле ссылки 26px с ореолом при фокусе (Tab), список пайплайнов в одной рамке; `Cmd+Shift+N` — диалог на сером фоне с тяжёлой тенью.

- [ ] **Step 10: Commit**

```bash
git add app/src/shared/ui
git commit -m "feat(web): примитивы «нативный desktop» — push-кнопки, поля, Segmented, карточки, теги, диалог"
```

---

### Task 5: Форма — сайдбар, шапка-тулбар, фон контента

**Files:**
- Modify: `app/src/app/form/router.tsx:52-64` (`Layout`)
- Modify: `app/src/app/form/report-route.tsx:48`
- Modify: `app/src/widgets/app-header/ui/AppHeader.tsx` (целиком)
- Modify: `app/src/features/build-by-link/ui/BuildByLink.tsx:40-42`
- Modify: `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx:17-26,59`
- Modify: `app/src/entities/saved-project/ui/SavedProjectItem.tsx` (целиком)
- Modify: `app/src/entities/history-entry/ui/HistoryEntryItem.tsx:12-31`

**Interfaces:**
- Consumes: токены `bg-chrome`, `bg-sidebar` (задача 3), `Button` (задача 4).

- [ ] **Step 1: `Layout` в `app/src/app/form/router.tsx`**

Было:
```tsx
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto flex max-w-4xl flex-col gap-3 p-6">
            <Outlet />
          </div>
          <AddProjectDialog />
        </main>
      </div>
```
Стало:
```tsx
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        {/* контент на «хроме» окна, страница — одна inset-grouped карточка на всю ширину */}
        <main className="flex-1 overflow-y-auto bg-chrome">
          <div className="flex min-h-full flex-col gap-3 px-5 py-4">
            <Outlet />
          </div>
          <AddProjectDialog />
        </main>
      </div>
```

- [ ] **Step 2: Шапка отчёта в окне, `app/src/app/form/report-route.tsx`**

Было: `<div className="flex items-center gap-3 border-b bg-card px-4 py-2">`
Стало: `<div className="flex items-center gap-3 border-b bg-chrome px-4 py-1.5">`

- [ ] **Step 3: `app/src/widgets/app-header/ui/AppHeader.tsx`**

```tsx
import { Link } from '@tanstack/react-router'
import { SettingsIcon } from 'lucide-react'
import { useTranslation } from '../../../shared/i18n'
import { buttonVariants } from '../../../shared/ui/button'
import { BuildByLink } from '../../../features/build-by-link'
import { TokenBlock } from '../../../features/manage-token'

/** Тулбар окна: поле ссылки, «Построить», шестерёнка. */
export function AppHeader() {
  const { t } = useTranslation()
  return (
    <header className="flex items-start gap-3 border-b bg-chrome px-4 py-2">
      <div className="min-w-0 flex-1">
        <BuildByLink tokenBlock={(host, retry) => <TokenBlock host={host} onSaved={retry} />} />
      </div>
      <Link to="/settings" aria-label={t('form.settingsButton')} className={buttonVariants({ variant: 'ghost', size: 'icon' })}>
        <SettingsIcon />
      </Link>
    </header>
  )
}
```

- [ ] **Step 4: Поле ссылки без видимой подписи, `app/src/features/build-by-link/ui/BuildByLink.tsx`**

Было:
```tsx
        <label htmlFor={id} className="text-sm font-medium">
          {t('form.link.label')}
        </label>
```
Стало (подпись остаётся для читалки, текст поля задаёт плейсхолдер — спека, § 4):
```tsx
        <label htmlFor={id} className="sr-only">
          {t('form.link.label')}
        </label>
```

- [ ] **Step 5: Сайдбар, `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx`**

В `Section` заменить `<h2 className="text-sm font-semibold">{title}</h2>` на
```tsx
        <h2 className="px-2 text-[11px] font-semibold text-muted-foreground">{title}</h2>
```
В `ProjectsSidebar` заменить `<aside className="flex w-72 shrink-0 flex-col gap-5 overflow-y-auto border-r bg-card p-4">` на
```tsx
    <aside className="flex w-60 shrink-0 flex-col gap-5 overflow-y-auto border-r bg-sidebar px-2.5 py-3">
```

- [ ] **Step 6: `app/src/entities/saved-project/ui/SavedProjectItem.tsx`**

```tsx
import { XIcon } from 'lucide-react'
import type { SavedProject } from '../../../shared/api'
import { cn } from '../../../shared/lib/cn'
import { useTranslation } from '../../../shared/i18n'

type Props = { project: SavedProject; active: boolean; onOpen: () => void; onRemove: () => void }

// крестик — вне <button>: вложенные кнопки недопустимы
// активный пункт — как выделение в source list macOS: синяя подложка, белый текст
export function SavedProjectItem({ project, active, onOpen, onRemove }: Props) {
  const { t } = useTranslation()
  return (
    <li className="group flex items-center gap-1">
      <button
        type="button"
        aria-current={active ? 'page' : undefined}
        onClick={onOpen}
        className={cn(
          'flex min-w-0 flex-1 flex-col rounded-md px-2 py-1 text-left focus-ring',
          active ? 'bg-primary text-primary-foreground' : 'hover:bg-accent',
        )}
      >
        <span translate="no" className="truncate text-sm">
          {project.name}
        </span>
        <span translate="no" className={cn('truncate text-[11px]', active ? 'text-primary-foreground/75' : 'text-muted-foreground')}>
          {project.host}
        </span>
      </button>
      <button
        type="button"
        aria-label={t('form.sidebar.removeProject', { name: project.name })}
        onClick={onRemove}
        className="rounded-md p-1 text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 hover:bg-accent focus-visible:opacity-100"
      >
        <XIcon className="size-4" />
      </button>
    </li>
  )
}
```

- [ ] **Step 7: `HistoryEntryItem` в `app/src/entities/history-entry/ui/HistoryEntryItem.tsx`**

Заменить разметку кнопки:
```tsx
    <button
      type="button"
      onClick={onOpen}
      className="flex min-w-0 flex-1 flex-col gap-0.5 rounded-md px-2 py-1 text-left focus-ring hover:bg-accent"
    >
      <span translate="no" className="truncate text-sm">
        {label}
      </span>
      <span className="flex gap-2 text-[11px] text-muted-foreground">
        <span translate="no" className="truncate">
          {entry.host}
        </span>
        <span className="shrink-0">{dateTime(entry.at, locale)}</span>
      </span>
    </button>
```

- [ ] **Step 8: Проверить**

Run: `cd app && npm run typecheck && npm run lint:fsd; cd -`
В браузере `http://localhost:5173/` рядом с `http://localhost:5174/?variant=native`: сайдбар серый 15rem с синим активным проектом, шапка серая с полем и кнопкой в одну линию (подписи «Вставь ссылку…» над полем нет), контент на сером, карточка проекта белая на всю ширину. Tab с поля ссылки → «Построить» → шестерёнка → пункты сайдбара, фокус виден. Тёмная тема (`classList.add('dark')`): сайдбар `#242426`, шапка `#2b2b2d`.

- [ ] **Step 9: Commit**

```bash
git add app/src/app/form app/src/widgets/app-header app/src/features/build-by-link app/src/widgets/projects-sidebar app/src/entities/saved-project app/src/entities/history-entry
git commit -m "feat(web): форма — source-list сайдбар, шапка-тулбар, контент на хроме окна"
```

---

### Task 6: Экран проекта, настройки, диалог — подписи, списки, Segmented

**Files:**
- Modify: `app/src/pages/project/ui/ProjectPage.tsx:22`
- Modify: `app/src/features/select-branch/ui/BranchSelect.tsx` (span подписи)
- Modify: `app/src/features/build-aggregate/ui/LastField.tsx:14-16`
- Modify: `app/src/features/build-aggregate/ui/StatusFilter.tsx:12`
- Modify: `app/src/features/manage-token/ui/HostSelect.tsx:16-18`
- Modify: `app/src/features/manage-token/ui/TokenBlock.tsx:30-32`, `TokenForm.tsx:25-27,31-33`
- Modify: `app/src/entities/pipeline/ui/PipelineRow.tsx:24-30`
- Modify: `app/src/entities/project/ui/ProjectRow.tsx:10-15`
- Modify: `app/src/pages/settings/ui/SettingsPage.tsx:26-47`
- Modify: `app/src/features/choose-theme/ui/ThemeSwitch.tsx` (целиком)
- Modify: `app/src/widgets/language-switch/ui/LanguageSwitch.tsx` (целиком)

**Interfaces:**
- Consumes: `labelClasses` из `shared/ui/input`, `Segmented` из `shared/ui/segmented` (задача 4).

- [ ] **Step 1: Подписи полей**

В каждом файле импортировать `labelClasses` из `'../../../shared/ui/input'` (там, где уже импортируется `Input`, дописать в тот же импорт) и заменить класс подписи `text-sm font-medium` на `labelClasses`:

`BranchSelect.tsx`:
```tsx
      <span aria-hidden className={labelClasses}>
        {t('form.branch.label')}
      </span>
```
`LastField.tsx`:
```tsx
      <label htmlFor={id} className={labelClasses}>
        {t('form.aggregate.last')}
      </label>
```
`StatusFilter.tsx` (импорт `import { labelClasses } from '../../../shared/ui/input'` и `import { cn } from '../../../shared/lib/cn'`):
```tsx
      <legend className={cn('mb-1.5', labelClasses)}>{t('form.aggregate.statuses')}</legend>
```
и у обоих `<input type="checkbox">` там же — `className="size-3.5 accent-primary"` (нативные чекбоксы 14×14, спека, § 3).
`HostSelect.tsx`:
```tsx
      <label htmlFor={id} className={labelClasses}>
        {t('form.host.label')}
      </label>
```
`TokenBlock.tsx` — подпись `form.host.token`; `TokenForm.tsx` — подписи `form.host.label` и `form.host.token`: тот же `className={labelClasses}`. Абзац `form.token.need` остаётся `text-sm font-medium`.

Текстовые ссылки — на токене `link` (спека, § 2: `primary` как цвет текста не проходит 4,5:1 на `chrome` и в тёмной теме): в `TokenBlock.tsx` у `<a href={createUrl}>` класс `text-link hover:underline` вместо `text-primary hover:underline`; в `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx` у кнопки «Найти в списке проектов хоста» — `self-start text-sm text-link hover:underline`.

- [ ] **Step 2: Заголовок проекта, `app/src/pages/project/ui/ProjectPage.tsx`**

Было: `<h1 translate="no" className="truncate text-xl font-semibold">`
Стало: `<h1 translate="no" className="truncate text-[17px] font-semibold">`

- [ ] **Step 3: Строка пайплайна, `app/src/entities/pipeline/ui/PipelineRow.tsx`**

Заменить `<li className="flex flex-col">` и классы кнопки:
```tsx
    <li className="flex flex-col even:bg-muted">
      <button
        type="button"
        disabled={busy}
        onClick={onOpen}
        className="grid w-full grid-cols-[4rem_5.5rem_1fr_9rem_8.5rem_4.5rem] items-center gap-3 px-2.5 py-1 text-left text-[12.5px] focus-ring hover:bg-selected disabled:opacity-60"
      >
```
(`8.5rem` для «52 минуты назад» без переноса — спека, § 4; остальное содержимое кнопки без изменений.)

- [ ] **Step 4: Строка проекта в диалоге, `app/src/entities/project/ui/ProjectRow.tsx`**

```tsx
    <li className="even:bg-muted">
      <button
        type="button"
        onClick={onOpen}
        className="flex w-full flex-col gap-0.5 px-2.5 py-1.5 text-left focus-ring hover:bg-selected"
      >
```

- [ ] **Step 5: Настройки, `app/src/pages/settings/ui/SettingsPage.tsx`**

Заголовок: `<h1 className="text-[17px] font-semibold">`. Заголовки секций `h2`: `className={labelClasses}` (импорт из `shared/ui/input`). Список хостов:
```tsx
        <ul className="flex flex-col overflow-hidden rounded-lg border empty:hidden">
          {hosts.map((h) => (
            <li key={h.host} className="flex items-center gap-3 px-2.5 py-1.5 text-sm even:bg-muted">
              <span translate="no" className="flex-1 truncate">
                {h.host}
              </span>
              <span className="text-[11px] text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              {h.source === 'keychain' && <RemoveTokenButton host={h.host} />}
            </li>
          ))}
        </ul>
```

- [ ] **Step 6: `app/src/features/choose-theme/ui/ThemeSwitch.tsx`**

```tsx
import type { Theme } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { applyTheme } from '../../../shared/lib/theme'
import { Segmented } from '../../../shared/ui/segmented'
import { useSettings, useUpdateSettings } from '../../../entities/settings'

const THEMES: Theme[] = ['system', 'light', 'dark']

/** Тема применяется сразу и сохраняется в settings.json. */
export function ThemeSwitch() {
  const { t } = useTranslation()
  const { data } = useSettings()
  const update = useUpdateSettings()
  const current = data?.theme ?? 'system'
  const choose = async (theme: Theme) => {
    applyTheme(theme)
    await update({ theme })
  }
  return (
    <Segmented
      label={t('form.settings.theme')}
      value={current}
      options={THEMES.map((theme) => ({ value: theme, label: t(`form.settings.themes.${theme}`) }))}
      onChange={(theme) => void choose(theme)}
    />
  )
}
```

- [ ] **Step 7: `app/src/widgets/language-switch/ui/LanguageSwitch.tsx`**

```tsx
import { useUpdateSettings } from '../../../entities/settings'
import type { Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { Segmented } from '../../../shared/ui/segmented'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: форма перерисовывается сразу, ядро запоминает язык и пересобирает меню. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  const update = useUpdateSettings()
  const choose = async (locale: Locale) => {
    await changeLocale(locale)
    await update({ locale })
  }
  return (
    <Segmented
      label={t('form.language')}
      value={i18n.language as Locale}
      options={LOCALES.map((locale) => ({ value: locale, label: locale.toUpperCase(), lang: locale }))}
      onChange={(locale) => void choose(locale)}
    />
  )
}
```

- [ ] **Step 8: Проверить**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales; cd -`
В браузере: экран проекта — подписи «ВЕТКА», «СТАТУСЫ», «СКОЛЬКО ПАЙПЛАЙНОВ» 11px серые, список пайплайнов в рамке с zebra, hover голубой, «52 минуты назад» в одну строку. `Cmd+,` — настройки: хосты в рамке, тема и язык — сегментированные переключатели, клик меняет тему сразу. `Cmd+Shift+N` → «Найти в списке проектов хоста» — строки проектов с zebra.

- [ ] **Step 9: Commit**

```bash
git add app/src/pages app/src/features app/src/entities/pipeline app/src/entities/project app/src/widgets/language-switch
git commit -m "feat(web): экран проекта, настройки и диалог на новых примитивах — подписи, zebra-списки, Segmented"
```

---

### Task 7: Отчёт — шапка, «Куда направить силы», карточки стейджей, строки

**Files:**
- Modify: `app/src/widgets/report-header/ui/ReportHeader.tsx:14-18` (`Chip`)
- Modify: `app/src/widgets/report-header/ui/LanguageSwitch.tsx` (целиком)
- Modify: `app/src/features/keys-help/ui/KeysHelp.tsx:16-18`
- Modify: `app/src/features/tree-switch/ui/BackToAggregate.tsx:12`
- Modify: `app/src/widgets/hotspots/ui/Hotspots.tsx:36-39`
- Modify: `app/src/widgets/waterfall/ui/StageCard.tsx` (целиком)
- Modify: `app/src/widgets/waterfall/ui/Row.tsx:51-58`
- Modify: `app/src/entities/node/ui/NodeLane.tsx:10,109`
- Modify: `app/src/entities/node/ui/DepTag.tsx:12-16`
- Modify: `app/src/entities/node/ui/StabilityPill.tsx:6`

**Interfaces:**
- Consumes: `Badge chip*` (задача 4), `Segmented size="xs"` (задача 4).

- [ ] **Step 1: Чипы шапки, `ReportHeader.tsx`**

```tsx
const Chip = ({ variant = 'chip', children }: { variant?: 'chip' | 'chipCrit' | 'chipBad'; children: ReactNode }) => (
  <Badge variant={variant} className="px-2 py-0.5 text-[12.5px]">
    {children}
  </Badge>
)
```

- [ ] **Step 2: RU/EN шапки, `app/src/widgets/report-header/ui/LanguageSwitch.tsx`**

```tsx
import type { Locale } from '../../../shared/api'
import { changeLocale, useTranslation } from '../../../shared/i18n'
import { Segmented } from '../../../shared/ui/segmented'

const LOCALES: Locale[] = ['ru', 'en']

/** RU/EN: язык меняется на месте, состояние просмотра не теряется. */
export function LanguageSwitch() {
  const { t, i18n } = useTranslation()
  return (
    <Segmented
      size="xs"
      label={t('report.language')}
      value={i18n.language as Locale}
      options={LOCALES.map((locale) => ({ value: locale, label: locale.toUpperCase(), lang: locale }))}
      onChange={(locale) => void changeLocale(locale)}
    />
  )
}
```

- [ ] **Step 3: Кнопка `?`, `KeysHelp.tsx`**

Было: `<Button variant="outline" size="icon" className="size-6 rounded-full text-muted-foreground" aria-label={t('report.help')}>`
Стало: `<Button variant="outline" size="icon" className="size-[22px] text-muted-foreground" aria-label={t('report.help')}>`

- [ ] **Step 4: «← к агрегату», `BackToAggregate.tsx`**

Было: `<Button variant="outline" className="h-auto rounded-full px-2.5 py-[3px] text-[12.5px] font-normal" onClick=…>`
Стало: `<Button variant="outline" size="sm" className="text-[12.5px] font-normal" onClick=…>`

- [ ] **Step 5: `Hotspots.tsx`**

Было:
```tsx
    <Card className="mt-px mb-2.5 gap-0 overflow-hidden rounded-[10px] border-0 p-0 pb-1.5 shadow-none ring-1 ring-crit-soft">
      <h2 className="m-0 bg-crit-bg px-3 py-[7px] text-[12.5px] font-semibold text-crit-fg">{t('report.hotspots.title')}</h2>
      {tree.hotspots.map((h) => (
        <button key={`${h.id}:${h.kind}`} type="button" className="flex w-full cursor-pointer items-baseline gap-2 px-3 py-1 text-left hover:bg-accent" onClick={() => reveal(h.id)}>
```
Стало (оранжевая точка вместо оранжевой плашки — спека, § 5):
```tsx
    <Card className="mt-px mb-2.5 gap-0 overflow-hidden p-0 pb-1.5">
      <h2 className="m-0 flex items-center gap-2 px-3 pt-2 pb-0.5 text-[13px] font-semibold">
        <span aria-hidden className="size-2 rounded-full bg-crit" />
        {t('report.hotspots.title')}
      </h2>
      {tree.hotspots.map((h) => (
        <button key={`${h.id}:${h.kind}`} type="button" className="flex w-full cursor-pointer items-baseline gap-2 px-3 py-[3px] text-left hover:bg-accent" onClick={() => reveal(h.id)}>
```

- [ ] **Step 6: `StageCard.tsx`**

```tsx
import type { ReactNode } from 'react'
import { cn } from '../../../shared/lib/cn'
import { Card } from '../../../shared/ui/card'

/** Карточка стейджа; `cont` — продолжение после строк downstream-пайплайна, без заголовка. */
export function StageCard({ cont, children }: { cont: boolean; children: ReactNode }) {
  return <Card className={cn('mt-px mb-2.5 gap-0 overflow-hidden p-0', cont && '-mt-[9px] rounded-t-none')}>{children}</Card>
}
```

- [ ] **Step 7: Строки, `Row.tsx`**

Было:
```tsx
      className={cn(
        GRID,
        'group scroll-my-10 cursor-pointer items-stretch focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring',
        stage ? 'h-8 border-b border-secondary' : 'h-[26px]',
        selected ? 'bg-selected' : 'hover:bg-accent',
```
Стало (строка стейджа на `muted`, zebra у джоб, выделение — синее с белым текстом, как в списках macOS; `**:` красит все вложенные подписи, иначе `text-muted-foreground`/`text-crit-fg` терялись бы на синем):
```tsx
      className={cn(
        GRID,
        'group scroll-my-10 cursor-pointer items-stretch focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring',
        stage ? 'h-8 border-b bg-muted' : 'h-[26px] odd:not-hover:bg-muted/50',
        selected ? 'bg-primary text-primary-foreground **:text-primary-foreground focus-visible:outline-primary-foreground' : 'hover:bg-accent',
```

- [ ] **Step 8: Полоски и подложка подписей стейджа, `NodeLane.tsx`**

Было: `const THICK = \`${BAR} top-[calc(50%-5px)] h-2.5 rounded-[5px]\``
Стало: `const THICK = \`${BAR} top-[calc(50%-6px)] h-3 rounded-[3px]\``

В `StageMeta` было: `<div className="absolute inset-y-0 right-3 z-[1] flex items-center gap-3 bg-card pl-1.5 text-[11px] whitespace-nowrap text-muted-foreground group-data-[selected]:bg-selected">`
Стало: `<div className="absolute inset-y-0 right-3 z-[1] flex items-center gap-3 bg-muted pl-1.5 text-[11px] whitespace-nowrap text-muted-foreground group-data-[selected]:bg-primary">`

- [ ] **Step 9: `DepTag.tsx`**

Было: `'absolute top-[calc(50%-8px)] right-1.5 z-[1] bg-card pl-1.5 text-[11px] leading-4 font-semibold whitespace-nowrap group-data-[selected]:bg-selected'`
Стало: `'absolute top-[calc(50%-8px)] right-1.5 z-[1] bg-card pl-1.5 text-[11px] leading-4 font-semibold whitespace-nowrap group-data-[selected]:bg-primary'`

- [ ] **Step 10: `StabilityPill.tsx`**

Было: `const PILL = 'px-[7px] py-0 text-[10.5px] leading-4 font-normal'`
Стало (в выделенной строке — полупрозрачная белая подложка, спека, § 5):
```tsx
const PILL = 'px-[7px] py-0 text-[10.5px] leading-4 group-data-[selected]:bg-white/20 group-data-[selected]:text-primary-foreground'
```

- [ ] **Step 11: Проверить**

Run: `cd app && npm run typecheck && npm run lint:fsd; cd -`
В браузере `http://localhost:5173/report.html?fixture=aggregate` рядом с `http://localhost:5174/report.html?fixture=aggregate&variant=native`: чипы шапки — серые теги без рамок, «ретраи» на розовом, «сэкономить» на кремовом; RU/EN — сегменты; «Куда направить силы» — белая карточка с оранжевой точкой; стейджи — карточки 10px с hairline, заголовок стейджа серый, джобы через одну на `muted/50`; клик по `build:static` — строка синяя, текст и метка «стабильна» белые, фокус-обводка белая; ↑/↓ двигают выделение, ←/→ сворачивают, Esc снимает. Полоски 12px.

- [ ] **Step 12: Commit**

```bash
git add app/src/widgets/report-header app/src/features/keys-help app/src/features/tree-switch app/src/widgets/hotspots app/src/widgets/waterfall app/src/entities/node
git commit -m "feat(web): отчёт — теги шапки, карточки стейджей, zebra и синее выделение строк"
```

---

### Task 8: Панель деталей — инспектор

**Files:**
- Modify: `app/src/widgets/detail-panel/ui/DetailPanel.tsx:22`
- Modify: `app/src/widgets/detail-panel/ui/Verdict.tsx:45`
- Modify: `app/src/widgets/detail-panel/ui/Kpis.tsx:5-13`
- Modify: `app/src/widgets/detail-panel/ui/Section.tsx` (целиком)
- Modify: `app/src/widgets/detail-panel/ui/GitlabLink.tsx:7`, `TopPipelines.tsx:28,45` (`text-primary` → `text-link`)

**Interfaces:**
- Consumes: `bg-chrome`, `text-link` (задача 3).

- [ ] **Step 1: `DetailPanel.tsx`**

Было: `<aside aria-labelledby="panel-title" className="fixed inset-y-0 right-0 z-[3] w-[min(420px,100vw)] overflow-x-hidden overflow-y-auto overscroll-contain border-l bg-card px-[18px] pt-3.5 pb-6 shadow-[-4px_0_16px_rgb(0_0_0/0.08)]">`
Стало: `<aside aria-labelledby="panel-title" className="fixed inset-y-0 right-0 z-[3] w-[min(420px,100vw)] overflow-x-hidden overflow-y-auto overscroll-contain border-l bg-chrome px-[18px] pt-3.5 pb-6">`

- [ ] **Step 2: `Verdict.tsx`**

Было: `<p className="mb-3 rounded-lg bg-secondary px-2.5 py-2">`
Стало: `<p className="mb-3 rounded-lg border bg-card px-2.5 py-2">`

- [ ] **Step 3: `Kpi` в `Kpis.tsx`**

```tsx
function Kpi({ name, value, small }: { name: string; value: string; small?: string }) {
  return (
    <div className="min-w-0 rounded-lg border bg-card px-2.5 py-2 whitespace-nowrap">
      <small className="block text-[11px] text-muted-foreground">{name}</small>
      <b className="mt-0.5 block text-[17px] font-semibold">{value}</b>
      {small && <i className="text-[11px] text-muted-foreground not-italic">{small}</i>}
    </div>
  )
}
```

- [ ] **Step 4: `Section.tsx`**

```tsx
import type { ReactNode } from 'react'

export const Section = ({ title, children }: { title: string; children: ReactNode }) => (
  <>
    <h3 className="mt-4 mb-1.5 text-[11px] font-semibold tracking-[.02em] text-muted-foreground">{title}</h3>
    {children}
  </>
)
```

- [ ] **Step 5: Ссылки панели на токене `link`**

В `GitlabLink.tsx` заменить `cn('text-primary', className)` на `cn('text-link', className)`. В `TopPipelines.tsx` заменить оба `text-primary` (кнопка `OpenTree` с именем пайплайна и кнопка «Все N») на `text-link`. Причина — спека, § 2: `primary` как цвет текста на `chrome` и в тёмной теме не проходит 4,5:1.

- [ ] **Step 6: Проверить**

Run: `cd app && npm run typecheck; cd -`
В браузере: клик по строке — панель на сером без тени с hairline слева, вывод и три KPI — белые плитки с рамкой, подписи KPI и заголовки секций без капса. Тёмная тема: панель `#2b2b2d`, плитки `#262626`. `×` и Esc закрывают. Ссылки «GitLab ↗» и `#N` — синие `link`.

- [ ] **Step 7: Commit**

```bash
git add app/src/widgets/detail-panel
git commit -m "feat(web): панель деталей как инспектор — на хроме окна, плитки с hairline"
```

---

### Task 9: Приёмка по спеке, § 7

**Files:**
- Нет новых; при расхождениях — правки в файлах задач 3–8.

- [ ] **Step 1: Сборка и чистота бандлов**

Run: `cd app && npm run typecheck && npm run lint:fsd && npm run check:locales && npm run build && grep -c "data-variant\|mock: нет команды" dist-report/report.html dist/assets/*.js; cd -`
Expected: всё зелёное; `grep` печатает `0` для каждого файла.

- [ ] **Step 2: Скриншоты против эталона**

Для каждого экрана — светлая и тёмная тема (в консоли `document.documentElement.classList.toggle('dark')`), рядом с эталоном на 5174 с `?variant=native`:

| Экран | Реализация | Эталон |
|---|---|---|
| проект | `http://localhost:5173/` | `http://localhost:5174/?variant=native` |
| настройки | `http://localhost:5173/#/settings` | `http://localhost:5174/?variant=native#/settings` |
| диалог | `Cmd+Shift+N` на экране проекта | то же |
| отчёт с панелью | `http://localhost:5173/report.html?fixture=aggregate`, клик по `build:static` | `http://localhost:5174/report.html?fixture=aggregate&variant=native` |

Expected: раскладка и цвета совпадают; допустимые расхождения — ширина сайдбара (15rem) и полупиксельные тени. Расхождение по цвету `primary` (`#0a6fe6` вместо `#0a7cff`) — намеренное (контраст).

- [ ] **Step 3: Контраст**

Проверить калькулятором WCAG пары из спеки, § 7, в обеих темах: `muted-foreground` на `background`, `chrome`, `sidebar` и `selected`; `primary-foreground` на `primary`; `link` на `background`, `card` и `chrome`; `pill-*-fg` на `pill-*-bg`; `crit-fg` на `crit-bg`; `retry-fg` на `pill-bad-bg`; `dep-down` на `card`. Expected: все ≥ 4,5:1. Если пара не проходит — затемнить `*-fg` в `theme.css` и повторить.

- [ ] **Step 4: Клавиатура**

На форме: Tab по полю ссылки → «Построить» → шестерёнка → сайдбар → карточка проекта; фокус виден везде (синий ореол 3px или обводка). `Cmd+,`, `Cmd+Shift+N`, Esc в диалоге и на настройках. В отчёте: ↑/↓, Enter, ←/→, `[`/`]`, `0`, `Cmd`+колесо, Esc.

- [ ] **Step 5: Сохранённый отчёт с `file://`**

```bash
open app/dist-report/report.html
```
Expected: страница открывается (в dev-шаблоне блок данных пуст — покажет «в отчёте нет данных», это ожидаемо); в консоли нет ошибок CSP про стили или шрифты. Полная проверка — через `npm run tauri dev`, `Cmd+S` на отчёте и открытие файла в браузере: вид совпадает с окном.

- [ ] **Step 6: Запах кода и финальный коммит**

Пройти по `git diff main...HEAD` по `docs/agents/code-smells.md`. Если правки были — закоммитить:
```bash
git add -A app/src
git commit -m "style(web): правки по приёмке редизайна"
```
Закрыть задачу в GitHub Issues комментарием со ссылкой на ветку `proto/redesign-variants` и этот план; снять worktree эталона: `git worktree remove ../redisign-proto`.
