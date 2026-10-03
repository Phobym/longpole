# Отчёт на React/TS в одном HTML-файле с данными из Rust

Дата обращения ко всем источникам: 2026-09-25. Замеры сделаны в тот же день
на Vite 8.3.1, vite-plugin-singlefile 2.3.3, React/ReactDOM 19.3.0,
Preact 10.29.8.

## 1. Вывод

Держать один Vite-проект с общим `src/` и двумя конфигами:

- `vite.config.ts` собирает окно Tauri как обычный фронтенд в `dist/`;
- `vite.report.config.ts` собирает отчёт в один `dist-report/index.html`
  через `vite-plugin-singlefile`. Небольшой плагин после инлайна считает
  sha256 для `<script>` и `<style>` и вписывает в `<head>` мета-тег CSP
  без `unsafe-inline`.

Rust встраивает этот файл через `include_str!`. При экспорте он вставляет
данные блоком `<script type="application/json" id="report-data">` и
экранирует в JSON только `<`. Код отчёта не меняется от отчёта к отчёту,
поэтому хеш в CSP остаётся верным для любых данных. Типы для TS генерирует
ts-rs.

Размер. React 19 добавляет к отчёту около 214 КБ без сжатия и около 68 КБ в
gzip. Через `preact/compat` выходит около 18 КБ и 7,4 КБ. Сейчас шаблон
весит 45,7 КБ и 14,1 КБ в gzip. Переход с React на Preact — это только
alias в конфиге отчёта, так что начать можно с React.

Проверка в браузере (Chromium, Playwright, страница по http). Отчёт с CSP
из одних хешей рендерится, и ошибок в консоли нет. Работают инлайн-стили
React (`style={{…}}`) и данные с `</script><script>alert(1)</script>` и
U+2028: скрипт не внедряется, текст выводится как есть. Если испортить хеш,
скрипт блокируется, и в консоли появляется ошибка CSP. Вариант с `file://`
не проверен: Playwright MCP блокирует `file:`. Подробнее в разделе 6.

## 2. Сборка в один файл: Vite + vite-plugin-singlefile

Штатный Vite один файл не соберёт. `build.assetsInlineLimit` только
переводит ассеты меньше порога в base64, по умолчанию порог 4 КиБ.
`build.cssCodeSplit: false` сводит весь CSS в один файл, но файл остаётся
отдельным. Встроить JS и CSS в HTML Vite не умеет
([vite.dev/config/build-options](https://vite.dev/config/build-options)).
Эту работу делает плагин.

Что включает `useRecommendedBuildConfig` (по умолчанию `true`), по исходнику
[`src/index.ts`](https://github.com/richardtallent/vite-plugin-singlefile/blob/main/src/index.ts):

- `assetsInlineLimit = () => true`: в base64 уходят все ассеты;
- `cssCodeSplit = false`, `base = "./"`, `assetsDir = ""`;
- `chunkSizeWarningLimit = 100000000`;
- на Vite 8 (Rolldown) `output.codeSplitting = false`, на Vite до 8
  `output.inlineDynamicImports = true`. Поэтому `import()` не создаёт
  отдельный чанк, а попадает в тот же бандл.

После этого в `generateBundle` плагин заменяет `<script src>` и
`<link rel=stylesheet>` на инлайновые теги. По умолчанию он удаляет
встроенные файлы (`deleteInlinedFiles: true`). С
`removeViteModuleLoader: true` плагин вырезает загрузчик модулей Vite
([исходник](https://github.com/richardtallent/vite-plugin-singlefile/blob/main/src/index.ts)).
Плагин поддерживает Vite `^5.4.21 || ^6 || ^7 || ^8`
([package.json](https://github.com/richardtallent/vite-plugin-singlefile/blob/main/package.json)).

Ограничения:

- Одна HTML-точка входа. Цитата из README: «will not work or will not be
  optimized for apps that require multiple "entry points"», и запросы на
  это закрываются как `wontfix`
  ([README](https://github.com/richardtallent/vite-plugin-singlefile#readme)).
  Отсюда отдельный конфиг для отчёта, см. раздел 4.
- Файлы из `public/` не встраиваются. SVG-файлы Vite тоже не встраивает:
  нужен загрузчик SVG или SVG прямо в разметке
  ([README, Caveats](https://github.com/richardtallent/vite-plugin-singlefile#caveats)).
  Для отчёта это не мешает: сейчас весь рендер идёт через инлайновую
  разметку.
- Воркеры. Плагин встраивает только чанки `.js` и `.css`, а остальные файлы
  оставляет рядом с пометкой «asset not inlined»
  ([исходник](https://github.com/richardtallent/vite-plugin-singlefile/blob/main/src/index.ts)).
  Воркер через `new Worker(new URL(…))` станет отдельным файлом. Встроить
  его можно только через `import W from './w?worker&inline'`: Vite
  встраивает такой воркер строкой base64
  ([vite.dev/guide/features#web-workers](https://vite.dev/guide/features#web-workers)).
  Но это конфликтует со строгим CSP из раздела 6. Воркер из blob или data
  URL требует `worker-src blob:`. Сейчас отчёту воркеры не нужны.
- Vite только транспилирует TS, типы не проверяет. Нужен
  `isolatedModules: true`, а в сборке стоит отдельно запускать `tsc --noEmit`
  ([vite.dev/guide/features#typescript](https://vite.dev/guide/features#typescript)).

## 3. Размер: React 19 и Preact

Первоисточники не публикуют размер React в КБ. Страница preactjs.com
заявляет только «Fast 3kB alternative to React»
([preactjs.com](https://preactjs.com/)). Поэтому цифры ниже — собственный
замер. Приложение: `createRoot` + `useState` + одна кнопка, сборка
`vite build` с плагином и `removeViteModuleLoader: true`, итоговый
`index.html`, gzip через `gzip -9`.

| Вариант | Без сжатия | gzip |
|---|---|---|
| React 19.3.0 + ReactDOM 19.3.0 (`react-dom/client`) | 214,3 КиБ (219,5 кБ по Vite) | 67,7 КБ |
| Preact 10.29.8 через `preact/compat` (alias) | 18,1 КиБ (18,6 кБ по Vite) | 7,4 КБ |
| Для сравнения: текущий `src/template.html` целиком | 45,7 КБ | 14,1 КБ |

Отчёт сохраняется на диск и открывается локально, поэтому смотреть надо на
размер без сжатия. React добавляет примерно 200 КБ к каждому
сохранённому файлу. Для одного офлайн-отчёта это приемлемо, но Preact меньше
примерно в 12 раз.

Как подключить Preact вместо React. Нужны alias `react` → `preact/compat`,
`react-dom` → `preact/compat`, `react/jsx-runtime` → `preact/jsx-runtime`.
В `tsconfig` нужны те же `paths` и `skipLibCheck: true`, потому что «some
React libraries make use of types that may not be provided by
`preact/compat`»
([preactjs.com/guide/v10/switching-to-preact](https://preactjs.com/guide/v10/switching-to-preact)).
Для `createRoot` в замере понадобился ещё alias `react-dom/client` →
`preact/compat/client`. Вариант с Preact в замере только собрался, в
браузере его не запускали.

Риск Preact. Совместимость заявлена для «React 15.x through 17.x, with
some 18.x and 19.x additions», а события идут через нативный
`addEventListener`, без синтетической системы событий
([preactjs.com/guide/v10/differences-to-react](https://preactjs.com/guide/v10/differences-to-react)).
Если общие компоненты используют API React 19, их придётся проверять в
обоих рантаймах. Поэтому рекомендация такая: начать с React в обоих местах
и переключить отчёт на `preact/compat`, только если размер файла станет
проблемой.

## 4. Один проект с двумя сборками

В одном `vite.config` можно задать несколько HTML-входов через
`build.rolldownOptions.input`
([vite.dev/guide/build#multi-page-app](https://vite.dev/guide/build#multi-page-app)).
Но singlefile-плагин с несколькими входами не работает (см. раздел 2). А
для окна Tauri встраивание всего в один файл не нужно. Поэтому схема такая:
один `package.json`, один `src/`, два конфиг-файла, и второй указывается
через `vite build -c <file>`
([vite.dev/guide/cli](https://vite.dev/guide/cli)).

Как делить код:

- компоненты и стили отчёта лежат в `src/report/`. Приложение импортирует
  их как обычные модули, чтобы показывать тот же водопад в окне;
- `src/report/main.tsx` — вход отчёта: читает `#report-data` и вызывает
  `createRoot`. Корень React-приложения обычно один, и `createRoot`
  вызывают один раз при старте
  ([react.dev/reference/react-dom/client/createRoot](https://react.dev/reference/react-dom/client/createRoot));
- `src/bindings/` — TS-типы, сгенерированные из Rust (раздел 7). Их
  используют обе сборки;
- CSS пишется обычными `.css`-импортами. В отчёте плагин встроит их в
  `<style>`, а в приложении Vite выпустит их отдельными файлами.

Раскладка:

```
desktop/
  index.html              # вход окна Tauri
  report.html             # вход отчёта
  vite.config.ts
  vite.report.config.ts
  src/app/…  src/report/…  src/bindings/…
  src-tauri/
```

Пример `vite.report.config.ts`. Идея проверена в замере: хеши из плагина
совпали с хешами, с которыми браузер пропустил скрипт и стиль.

```ts
import { defineConfig, type Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import { viteSingleFile } from 'vite-plugin-singlefile'
import { createHash } from 'node:crypto'

const sha = (s: string) =>
  `'sha256-${createHash('sha256').update(s, 'utf8').digest('base64')}'`

// После инлайна: считает хеши <script>/<style> и вписывает CSP первым тегом <head>.
function reportCsp(): Plugin {
  return {
    name: 'report-csp',
    enforce: 'post',
    generateBundle(_, bundle) {
      const html = bundle['report.html'] as { source: string }
      const src = String(html.source)
      const hashes = (tag: string) =>
        [...src.matchAll(new RegExp(`<${tag}[^>]*>([\\s\\S]*?)</${tag}>`, 'g'))]
          .map((m) => sha(m[1])).join(' ')
      const csp = `default-src 'none'; script-src ${hashes('script')}; ` +
        `style-src ${hashes('style')}; img-src data:`
      html.source = src.replace('<head>',
        `<head>\n<meta http-equiv="Content-Security-Policy" content="${csp}">`)
    },
  }
}

export default defineConfig({
  plugins: [react(), viteSingleFile({ removeViteModuleLoader: true }), reportCsp()],
  // для Preact добавить:
  // resolve: { alias: { react: 'preact/compat', 'react-dom/client': 'preact/compat/client',
  //   'react-dom': 'preact/compat', 'react/jsx-runtime': 'preact/jsx-runtime' } },
  build: {
    outDir: 'dist-report',
    rolldownOptions: { input: 'report.html' },
  },
})
```

Плагин должен стоять после `viteSingleFile` в массиве `plugins` (оба с
`enforce: 'post'`), иначе он увидит HTML до инлайна.

Скрипты в `package.json`:

```json
"build": "tsc --noEmit && vite build && vite build -c vite.report.config.ts"
```

## 5. Как Rust получает шаблон и подставляет данные

### Шаблон в бинарнике

`include_str!` читает файл на этапе компиляции, путь считается
относительно текущего `.rs`-файла, а результат — `&'static str`
([doc.rust-lang.org/std/macro.include_str.html](https://doc.rust-lang.org/std/macro.include_str.html)):

```rust
const REPORT_TEMPLATE: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../dist-report/report.html"));
```

Порядок сборки. `beforeBuildCommand` — это «a shell command to run before
`tauri build` kicks in», для `tauri dev` есть отдельная `beforeDevCommand`
([v2.tauri.app/reference/config](https://v2.tauri.app/reference/config/)).
Типовая связка для Vite: `beforeBuildCommand: "npm run build"`,
`frontendDist: "../dist"`
([v2.tauri.app/start/frontend/vite](https://v2.tauri.app/start/frontend/vite/)).
Значит, при `tauri build` и `tauri dev` шаблон появится раньше, чем
запустится `cargo`. Но при голом `cargo build` или `cargo test` эти хуки не
выполняются, и `include_str!` упадёт на отсутствующем файле. Чтобы ошибка
была понятной и пересборка шла при изменении шаблона, нужен `build.rs`:

```rust
fn main() {
    let p = "../dist-report/report.html";
    println!("cargo::rerun-if-changed={p}");
    assert!(std::path::Path::new(p).exists(), "нет {p}: сначала `npm run build`");
    tauri_build::build();
}
```

`rerun-if-changed` сверяет mtime файла
([doc.rust-lang.org/cargo/reference/build-scripts.html](https://doc.rust-lang.org/cargo/reference/build-scripts.html)).
Копировать шаблон в `OUT_DIR` не нужно: build.rs его только читает.

`dist-report/` нельзя класть внутрь `dist/`. Tauri рекурсивно встраивает
всё содержимое `frontendDist`
([v2.tauri.app/reference/config](https://v2.tauri.app/reference/config/)),
и шаблон оказался бы в бинарнике дважды.

### Подстановка JSON

Нужен `<script type="application/json">`, а не глобальная переменная.
Скрипт с типом, отличным от JavaScript, — это «data block», который «not
processed by the user agent»
([HTML, the script element](https://html.spec.whatwg.org/multipage/scripting.html#the-script-element)).
Отсюда два плюса:

1. Данные не исполняются. Код отчёта — один и тот же скрипт, поэтому его
   хеш вычисляется при сборке и подходит к любым данным. С `const REPORT =
   {…}` данные стали бы частью кода, и хеш пришлось бы считать для каждого
   отчёта в Rust. Замер подтверждает, что data block при
   `script-src 'sha256-…'` не блокируется.
2. Данные читаются через `JSON.parse(el.textContent)`, а не как литерал JS.
   Поэтому U+2028/U+2029 экранировать не нужно: в JSON-строках они
   допустимы.

Экранирование. Внутри `<script>` HTML-парсер ищет `<!--`, `<script` и
`</script` без учёта регистра. Спецификация советует экранировать их как
`\x3C…` ([HTML, restrictions for contents of script elements](https://html.spec.whatwg.org/multipage/scripting.html#restrictions-for-contents-of-script-elements)).
В JSON это делается одной заменой `<` на `<`. Она закрывает все три
последовательности, а `JSON.parse` вернёт исходный `<`. `serde_json` сам
экранирует только `"`, `\` и управляющие символы U+0000–U+001F
([serde_json, `ESCAPE` в ser.rs](https://github.com/serde-rs/json/blob/master/src/ser.rs)),
так что замена обязательна:

```rust
pub fn render(report: &Report) -> serde_json::Result<String> {
    let json = serde_json::to_string(report)?.replace('<', "\\u003c");
    // функция-заменитель не нужна: str::replace не интерпретирует `$`
    Ok(REPORT_TEMPLATE.replacen(
        "<!--__DATA__-->",
        &format!(r#"<script type="application/json" id="report-data">{json}</script>"#),
        1,
    ))
}
```

Маркер `<!--__DATA__-->` ставится в `<body>` файла `report.html`. Замер
показал, что Vite 8 с singlefile-плагином сохраняет его в выходном файле.
Порядок в разметке: data block идёт до скрипта
модуля. Скрипт с `type="module"` отложен и выполнится после разбора
документа, так что `getElementById` найдёт блок в любом случае. Нужен
тест: строка `</script><!--<script>` и U+2028 в данных проходят круговой
путь, как в текущем `test/`.

## 6. CSP для инлайнового скрипта

### Файл, открытый в браузере

Политику задаёт `<meta http-equiv="Content-Security-Policy">`, и мета-тег
должен стоять до скриптов. В мета-теге не работают `report-uri`,
`frame-ancestors` и `sandbox`
([CSP3 §3.3](https://www.w3.org/TR/CSP3/#meta-element)). Для инлайнового
кода подходит hash-source: хешируется текст элемента
([CSP3 §6.7.2](https://www.w3.org/TR/CSP3/#matching-algorithms)).

Рекомендация: хеши, без `unsafe-inline`. Политика из замера:

```
default-src 'none'; script-src 'sha256-…'; style-src 'sha256-…'; img-src data:
```

Результат в Chromium: отчёт рендерится, инлайновые стили React через
`style={{…}}` применяются (React ставит их через DOM, а не атрибутом в
разметке), в консоли 0 ошибок. Если испортить хеш, скрипт блокируется.
`default-src 'none'` заодно запрещает сетевые запросы (`connect-src`,
`img-src` кроме `data:`), а это и есть требование «без внешних запросов».

Не проверено:

- `file://`. Инлайновый модуль ничего не загружает, поэтому CORS-ограничения
  `file://` его не касаются. Но прогон шёл по http. Перед внедрением стоит
  один раз открыть файл двойным кликом в Chrome, Firefox и Safari;
- `style="…"` в статической разметке `report.html` потребовал бы
  `unsafe-inline` или `unsafe-hashes`. Его просто не надо писать.

### Внутри окна Tauri

Tauri подставляет `app.security.csp` во все HTML-файлы собранного
приложения. При компиляции он сам добавляет хеши локальных скриптов и
nonce для стилей: «Local scripts are hashed, styles and external scripts
are referenced using a cryptographic nonce»
([v2.tauri.app/security/csp](https://v2.tauri.app/security/csp/);
[config: `app.security.csp`, `dangerousDisableAssetCspModification`](https://v2.tauri.app/reference/config/)).
Для окна приложения это обычная сборка из `dist/`, и `unsafe-inline` не
нужен.

Рекомендация: в окне Tauri не загружать сохранённый single-file, а
показывать те же компоненты из `src/report/` внутри приложения, с данными
через команду Tauri. Тогда single-file — только формат экспорта, и его CSP
живёт в самом файле. Автоматика Tauri работает только с ассетами,
собранными при компиляции. Для HTML, сгенерированного в рантайме, хеши она
не добавит (вывод из формулировки «at compile time» в
[v2.tauri.app/security/csp](https://v2.tauri.app/security/csp/), не
проверено).

## 7. TS-типы из Rust

| | ts-rs | specta + specta-typescript | typeshare |
|---|---|---|---|
| Версия | 12.0.1, 2026-09-13 ([docs.rs](https://docs.rs/ts-rs/latest/ts_rs/)) | specta 2.0.0-rc.25 (стабильная 1.0.5 от 2023-07), specta-typescript 0.0.12 ([docs.rs](https://docs.rs/crate/specta/latest), [docs.rs](https://docs.rs/specta-typescript/latest/specta_typescript/)) | 1.0.5 ([docs.rs](https://docs.rs/typeshare/latest/typeshare/)) |
| Механизм | `#[derive(TS)]` + `#[ts(export)]`; файлы пишутся при `cargo test`, каталог задаёт `TS_RS_EXPORT_DIR` | `#[derive(Type)]` + вызов `Typescript::default().export_to(…)` из кода | `#[typeshare]` + отдельная CLI `typeshare-cli` парсит исходники ([github](https://github.com/1Password/typeshare)) |
| serde | фича `serde-compat` по умолчанию: `rename`, `rename_all`, `tag`, `content`, `untagged`, `skip`, `flatten`, `default` | через `specta_serde::Format` | да, по заявлению README |
| i64/u64 | по умолчанию `bigint`, меняется через `TS_RS_LARGE_INT` | есть приведение `BigInt` (поведение по умолчанию в документации не описано) | панический отказ: `panic!("64 bit types not allowed in Typeshare")`, предлагаются `U53`/`I54` ([typescript.rs](https://github.com/1Password/typeshare/blob/main/core/src/language/typescript.rs)) |
| Связь с Tauri | нет | `tauri-specta` типизирует ещё и команды | нет |

Рекомендация: ts-rs. Он стабилен, не требует внешней CLI и уважает serde.
Одна настройка: `serde_json` отдаёт `u64` как JSON-число, а ts-rs по
умолчанию пишет для него `bigint`. Это ложь в типах. Нужно выставить
`TS_RS_LARGE_INT=number` в `.cargo/config.toml` или помечать поля
`#[ts(type = "number")]`
([docs.rs/ts-rs](https://docs.rs/ts-rs/latest/ts_rs/)). Длительности в мс
и id джобов GitLab укладываются в 2^53.

specta стоит брать, только если понадобятся типы команд Tauri через
`tauri-specta`, и с учётом того, что 2.0 всё ещё RC. typeshare отпадает:
он требует отдельную CLI и отказывает на 64-битных целых.

## 8. Открытые вопросы

- Проверить отчёт при открытии через `file://` в трёх браузерах (раздел 6).
- Размер на реальном отчёте: замер выше — голый рантайм, к нему добавятся
  компоненты и данные.
