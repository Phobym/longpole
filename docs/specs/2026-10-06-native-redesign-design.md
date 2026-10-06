# Редизайн «Нативный desktop» — спека

Дата: 2026-10-06. Визуальный язык формы и отчёта меняется на «как системное окно macOS/Windows». Экраны, поведение, клавиши и тексты не меняются (паритет по issue #13). Выбор сделан по прототипу из четырёх вариантов (`app/src/app/prototype/`, ветка `proto/redesign-variants`): вариант C победил у «как сейчас», «Swiss» и «mono».

## 1. Цель и рамки

Приложение перестаёт выглядеть как shadcn по умолчанию (индиго-кнопка, серый фон с белыми карточками в тенях, пилюли, KPI-плитки) и выглядит как часть ОС: source-list сайдбар, тулбар, push-кнопки, inset-grouped карточки, инспектор справа, системный синий как единственный акцент.

Делается:

- новые токены цвета, радиуса и поверхностей в `shared/ui/theme.css`, светлая и тёмная тема;
- переработка примитивов `shared/ui`: Button, Input, Select, Card, Badge, Dialog + новый `Segmented`;
- раскладка формы: сайдбар, шапка-тулбар, фон контента, список пайплайнов, подписи полей;
- отчёт: шапка, «Куда направить силы», карточки стейджей, строки, выделение, панель деталей;
- экран настроек и диалог «Добавить проект» на тех же примитивах;
- мок Tauri-моста для dev в браузере остаётся как инструмент разработки.

Не делается: новые функции и экраны, изменение структуры панели или водопада, шрифты в бандле (отчёт — один файл с CSP, только системные), вибрация/прозрачность окна (Tauri `transparent`/vibrancy), стилизация под Windows отдельно, компоненты Agent Prism (вариант снят пользователем), автотесты фронтенда.

Правила uupm, которым спека обязана: контраст текста ≥ 4,5:1 в обеих темах, видимый фокус, `prefers-reduced-motion` не нарушается (анимаций не добавляем), иконки только lucide, `cursor: pointer` на кликабельном (уже есть).

## 2. Токены

`@theme static` в `theme.css`; значения светлой темы в `:root`, тёмной — в `:root.dark`. Домашние токены статусов и критического пути остаются с теми же именами, меняются значения. Новые токены — `--color-chrome` (фон «хрома» окна: тулбар, фон контента, инспектор, диалог) и `--color-sidebar`.

| Токен | Светлая | Тёмная | Где |
|---|---|---|---|
| background | `#ffffff` | `#1e1e1e` | фон отчёта и списков |
| foreground | `#1d1d1f` | `#f5f5f7` | текст |
| chrome (новый) | `#f5f5f7` | `#2b2b2d` | тулбар, фон контента формы, инспектор, диалог |
| sidebar (новый) | `#ececf0` | `#242426` | сайдбар |
| card | `#ffffff` | `#262626` | карточки, KPI, поповеры |
| popover | `#ffffff` | `#2c2c2e` | |
| primary | `#0a6fe6` | `#1f6fdc` | кнопка, выделение, ссылки; белый текст ≥ 4,5:1 |
| primary-foreground | `#ffffff` | `#ffffff` | |
| secondary | `#f2f2f7` | `#2c2c2e` | теги шапки отчёта, подложка сегментов |
| muted | `#f5f5f7` | `#2a2a2a` | строки стейджей, zebra |
| muted-foreground | `#6e6e73` | `#a1a1a6` | подписи (5,1:1 на белом, 4,7:1 на chrome) |
| accent | `#eeeef2` | `#333336` | hover |
| border | `#d8d8dc` | `#3a3a3c` | hairline |
| input | `#c9c9cf` | `#48484a` | рамки полей |
| ring | = primary | = primary | фокус |
| selected | `#dbe8ff` | `#1f3a5f` | hover строк списка пайплайнов и хостов |
| destructive | `#d0281f` | `#ff6961` | |
| ok / fail / warn / run / idle | `#34c759` / `#ff3b30` / `#ffcc00` / `#0a6fe6` / `#c7c7cc` | `#30d158` / `#ff453a` / `#ffd60a` / `#1f6fdc` / `#48484a` | полоски |
| queued / thin / wait | `#e5e5ea` / `#d1d1d6` / `#e5e5ea` | `#3a3a3c` / `#48484a` / `#3a3a3c` | |
| crit / crit-fg / crit-soft / crit-bg | `#ff9500` / `#b35f00` / `#ffd9a8` / `#fff7eb` | `#ff9f0a` / `#ffb84d` / `#5a3d10` / `#2a2416` | критический путь |
| over-b / retry-b / retry-fg | `#ffe2bf` / `#ffd0cd` / `#d0281f` | `#4a3a1c` / `#4a2422` / `#ff6961` | штриховки |
| dep-up / dep-down | `#0a6fe6` / `#1f8a3b` | `#64d2ff` / `#30d158` | связи |
| pill-ok bg/fg | `#e3f7e8` / `#1f8a3b` | `#1d3a27` / `#4ade80` | метки стабильности |
| pill-mid bg/fg | `#fff5cc` / `#8a6400` | `#3b3414` / `#ffd60a` | |
| pill-bad bg/fg | `#ffe3e1` / `#d0281f` | `#3f1f1d` / `#ff6961` | |
| pill-run bg/fg | `#e1eeff` / `#0a5fd0` | `#1c2d4a` / `#64d2ff` | |

`--radius: 0.5rem` (md = 6px — контролы; lg = 8px — KPI, список пайплайнов; xl = 12px — диалог). Карточки стейджей и формы — 10px, задаётся на месте. `theme-color` в `report.html` — `#ffffff` / `#1e1e1e`.

Шрифт: системный стек `-apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI Variable", "Segoe UI", system-ui, Roboto, sans-serif`, `-webkit-font-smoothing: antialiased`, базовый размер 13px (`html`), отчёт 12,5px как сейчас, `tabular-nums` как сейчас.

## 3. Примитивы `shared/ui`

- **Button**. Высота 26px (`sm` — 22px, `icon` — 26×26), радиус 6px, шрифт 13px/500. `default`: градиент `linear-gradient(#3a97ff, primary)`, тень `0 .5px 1px rgb(0 0 0/.25), inset 0 .5px 0 rgb(255 255 255/.25)`. `outline`: градиент `card → secondary`, рамка `rgb(0 0 0/.12)` (в тёмной `rgb(255 255 255/.1)`, градиент `#5a5a5e → #4a4a4e`), тень `0 .5px 1px rgb(0 0 0/.15)`. `ghost` и `secondary` — без градиента, как сейчас. `destructive` — на `destructive` без градиента.
- **Input** и `inputClasses` (Combobox). Высота 26px, радиус 6px, рамка `rgb(0 0 0/.18)` (тёмная: `rgb(255 255 255/.12)`, фон `#1a1a1a`), внутренняя тень `inset 0 .5px 1px rgb(0 0 0/.08)`, отступ 8px; фокус — рамка `primary` и ореол `0 0 0 3px rgb(10 111 230/.35)` вместо `ring-2`.
- **SelectTrigger**. Как `outline`-кнопка, 26px.
- **Segmented** (новый, `shared/ui/segmented.tsx`). `role="group"` с подложкой `secondary`, радиус 6px, отступ 1px; пункт 22px, радиус 5px, 12px; активный — `card`, тень `0 .5px 2px rgb(0 0 0/.2)`, 500. Заменяет пары ghost/secondary кнопок в ThemeSwitch и обоих LanguageSwitch.
- **Card**. Радиус 10px, рамка `rgb(0 0 0/.1)` (тёмная `rgb(255 255 255/.08)`), фон `card`, тень `0 1px 2px rgb(0 0 0/.04)`, отступ 16×18px.
- **Badge**. Радиус 4px у `ok/run/mid/bad` (вес 500); `chip` — без рамки, фон `secondary`, радиус 6px, отступ 2×9px; `chipCrit` — фон `crit-bg`, `chipBad` — фон `pill-bad-bg`.
- **DialogContent**. Радиус 12px, фон `chrome`, тень `0 20px 60px rgb(0 0 0/.3), 0 0 0 .5px rgb(0 0 0/.2)`.
- **Popover, Command** — без изменений, кроме радиуса из токена.
- Чекбоксы — нативные 14×14 (`accent-primary` остаётся).

## 4. Форма

- **Сайдбар** (`widgets/projects-sidebar`): ширина 15rem, фон `sidebar`, правая hairline, отступ 12×10px. Заголовки секций 11px/600 `muted-foreground`, без капса. Пункты (`SavedProjectItem`, `HistoryEntryItem`): радиус 6px, отступ 4×8px, имя 400; активный проект — фон `primary`, текст белый, подпись `rgb(255 255 255/.75)`; hover — `accent`.
- **Шапка** (`widgets/app-header`, `features/build-by-link`): фон `chrome`, нижняя hairline, отступ 8×16px, элементы по центру. Видимый label поля убирается, текст «Вставь ссылку на пайплайн или MR» остаётся плейсхолдером и `aria-label`. Шестерёнка без `mt-6`.
- **Контент** (`app/form/router.tsx` Layout): область под шапкой на `chrome`, без `max-w-4xl`, отступ 16×20px; страницы — одна Card.
- **Экран проекта**: h1 17px/600; подписи полей («Ветка», «Статусы», «Сколько пайплайнов») — 11px капсом `muted-foreground`, трекинг .02em (общий класс `labelClasses` в `shared/ui/input.tsx`).
- **Список пайплайнов** (`PagedList`, `PipelineRow`): `ul` без промежутков, рамка `border`, радиус 8px, `overflow: hidden`; строки без собственных рамок и радиуса, отступ 5×10px, 12,5px, чётные — `muted`, hover — `selected`, колонки `4rem 5.5rem 1fr 9rem 8.5rem 4.5rem` (чтобы «52 минуты назад» не переносилось).
- **Настройки**: та же Card; строки хостов — как строки списка пайплайнов (контейнер с рамкой, zebra); тема и язык — `Segmented`.
- **Диалог «Добавить проект»**: на `DialogContent`; поле и кнопки — примитивы.
- **Шапка отчёта в окне** (`report-route.tsx`): фон `chrome`.

## 5. Отчёт

- **Шапка** (`widgets/report-header`): h1 15px/600; чипы — `Badge chip` по § 3 (теги, без рамок); RU/EN — `Segmented`; `?` — `outline` 22px.
- **«Куда направить силы»** (`widgets/hotspots`): Card 10px с рамкой `rgb(0 0 0/.1)`, без оранжевого фона заголовка; заголовок 13px `foreground` с оранжевой точкой 8px слева; пункты — отступ 3×12px.
- **Карточки стейджей** (`StageCard`): радиус 10px, рамка `rgb(0 0 0/.1)` (тёмная `.08` белого), тень `0 1px 2px rgb(0 0 0/.04)`, отступ снизу 10px.
- **Строки** (`Row`): строка стейджа — фон `muted`, нижняя hairline; строки джоб — нечётные на `muted/50` (zebra); hover — `accent`; выделенная — фон `primary`, текст белый, подписи `rgb(255 255 255/.85)`, `StageMeta`/`DepTag` подложка `primary`, метки стабильности `rgb(255 255 255/.2)` с белым текстом; фокус — как сейчас.
- **Полоски** (`NodeLane`): толстая 12px, радиус 3px; остальное без изменений.
- **Панель деталей** (`DetailPanel`): фон `chrome`, левая hairline, без тени. `Verdict` и KPI — плитки `card` с рамкой `border`, радиус 8px; подписи KPI 11px без капса; заголовки секций (`Section`) 11px/600 `muted-foreground` без капса, трекинг .02em; ссылки `primary`.
- **Ось** (`Waterfall` sticky): фон `background`.

## 6. Реализация и чистка

- Токены и шрифт — `theme.css`; примитивы — `shared/ui/*`; остальное — классы в перечисленных компонентах. Прототипные селекторы не переносятся: каждое правило из `variants.css` для `native` становится утилитой в нужном компоненте.
- Удаляются `app/src/app/prototype/switcher.tsx` и `variants.css`, из обоих `main.tsx` — их подключение. `app/src/app/prototype/mock.ts` переезжает в `app/src/app/dev/mock.ts` и остаётся подключённым только в dev без Tauri.
- Прототип целиком коммитится в ветку `proto/redesign-variants` (не в `main`); ссылка на неё — в задаче редизайна в GitHub Issues.
- Спека Tauri (§ 12 «отступления») и README не трогаются: поведение не меняется.

## 7. Приёмка

- `npm run typecheck`, `npm run lint:fsd`, `npm run build` зелёные; в `dist-report/report.html` нет `data-variant` и мока.
- Скриншоты в браузере (`npm run dev`, мок) четырёх экранов — проект, настройки, диалог, отчёт с открытой панелью — в светлой и тёмной теме совпадают с прототипом `?variant=native` по раскладке и цветам.
- Контраст: `muted-foreground` на `background` и `chrome`, белый на `primary`, `pill-*-fg` на `pill-*-bg`, `crit-fg` на `crit-bg` — ≥ 4,5:1 в обеих темах (проверка любым калькулятором WCAG).
- Клавиатура: Tab-обход, фокус виден на кнопках, полях, строках отчёта и пунктах сайдбара; Esc, ↑/↓, ←/→, `[`/`]`, `0`, `Cmd+S`, `Cmd+,`, `Cmd+Shift+N` работают как до редизайна.
- Сохранённый HTML-отчёт открывается с `file://` в браузере с тем же видом (CSP на хешах не нарушен).
