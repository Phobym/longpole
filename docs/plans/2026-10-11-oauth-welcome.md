# Вход через OAuth на первом экране — план реализации

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Новый пользователь входит через GitHub/GitLab с первого экрана и сразу выбирает проект; пользователь с одним аккаунтом подключает второй из блока «Аккаунты» в сайдбаре или из диалога «Добавить проект».

**Architecture:** Только фронтенд, `core` и команды Tauri не меняются. `HomePage` выбирает шаг чистой функцией `homeStep(projects, hosts)`: приветствие, выбор проекта или переход на последний проект. Вход — новый компонент `ConnectAccount` в `features/manage-token` поверх существующих `useOauthLogin` и `TokenForm`; ожидание входа вынесено в `OauthWaiting` и общее с `OauthLogin`. Выбор проекта (`PickProject`) и диалог «Подключить аккаунт» (`ConnectDialog`) — новый виджет `widgets/connect-account`; стор открытия диалога — в `features/manage-token`, чтобы сайдбар и диалог «Добавить проект» открывали его без кросс-импорта.

**Tech Stack:** React 19, TanStack Router/Query, zustand, Radix (`radix-ui`), Tailwind 4, i18next; тесты — `node --test` с type stripping (Node 24), как `features/onboarding/model/pick.test.ts`.

Спека — `docs/specs/2026-10-11-oauth-welcome-design.md` (далее «спека, § N»). Правила репозитория: `docs/agents/code-smells.md` — проход по диффу перед каждым коммитом; комментарии и имена тестов — по-русски, как в соседнем коде. Все команды — из `app/`. Хук pre-commit (lefthook) гоняет `typecheck`, `lint:fsd`, `check:locales` и тесты; без `npm ci` в `app/` хуки молча пропускаются — перед первой задачей выполнить `npm ci`.

## Global Constraints

- `core`, `src-tauri`, `shared/api/schema/*` не меняются.
- Новые строки — во все 8 локалей `app/src/shared/i18n/locales/*.json`; `npm run check:locales` зелёный. Русский — на «ты», французский — на «vous», немецкий — безлично (инфинитив), как в соседних строках.
- Текст на цветной заливке — контраст не ниже 4,5:1 (комментарий палитры в `shared/ui/theme.css`). Поэтому кнопка GitLab — `outline` с оранжевым логотипом, а не белый текст на `#fc6d26` (≈2,9:1).
- FSD (`npm run lint:fsd`): слайсы одного слоя не импортируют друг друга; `features` не импортируют `widgets`.
- Ссылки во внешний браузер — `<a href="https://…" target="_blank" rel="noopener noreferrer">`: `https` уходит в системный браузер через `on_navigation`. Кнопок с `window.open` не добавлять.

## Файлы

| Файл | Что |
|---|---|
| `app/package.json`, `lefthook.yml`, `.github/workflows/ci.yml` | `test:onboarding` → общий `test` по `src/**/*.test.ts` |
| `app/src/pages/home/model/step.ts` (новый), `step.test.ts` (новый) | `homeStep` |
| `app/src/shared/i18n/locales/*.json` | ключи `form.welcome.*`, `form.connect.*`, `form.sidebar.accounts/connect`; новый текст `form.host.add`; удаление `form.empty.*`, `form.oauth.copy/open` |
| `app/src/shared/ui/provider-icon.tsx` (новый) | логотипы GitHub/GitLab |
| `app/src/features/manage-token/model/connect.ts` (новый) | стор диалога «Подключить аккаунт» |
| `app/src/features/manage-token/ui/OauthWaiting.tsx` (новый) | ожидание входа: код GitHub или «заверши в браузере» |
| `app/src/features/manage-token/ui/ConnectAccount.tsx` (новый) | кнопки провайдеров и «Другой сервер» |
| `app/src/features/manage-token/ui/OauthLogin.tsx` | ожидание — через `OauthWaiting` |
| `app/src/features/manage-token/ui/HostSelect.tsx` | «+ Другой аккаунт…» зовёт `onAddAccount` |
| `app/src/features/manage-token/index.ts` | экспорты |
| `app/src/entities/host/model/hosts.ts`, `index.ts` | удалить `pickAddHost` и `kind: 'add'` |
| `app/src/widgets/connect-account/` (новый) | `PickProject`, `ConnectDialog` |
| `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx` | `onAddAccount` для `HostSelect` |
| `app/src/app/form/router.tsx` | монтировать `ConnectDialog` |
| `app/src/pages/home/ui/HomePage.tsx` | приветствие и выбор проекта |
| `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx` | блок «Аккаунты» |
| `app/src/app/dev/mock.ts` | режим `?welcome`, код GitHub, новый хост после входа |

---

### Task 1: `homeStep` и общий скрипт тестов

**Files:**
- Create: `app/src/pages/home/model/step.ts`
- Test: `app/src/pages/home/model/step.test.ts`
- Modify: `app/package.json` (скрипт `test:onboarding`), `lefthook.yml` (команда `test-onboarding`), `.github/workflows/ci.yml:39`

**Interfaces:**
- Produces: `export type HomeStep = 'welcome' | 'pick' | 'redirect'`; `export function homeStep(projects: number, hosts: number): HomeStep` — из `pages/home/model/step.ts`.

- [ ] **Step 1: Обобщить скрипт тестов**

В `app/package.json` заменить строку

```json
    "test:onboarding": "node --test src/features/onboarding/model/pick.test.ts",
```

на

```json
    "test": "node --test 'src/**/*.test.ts'",
```

В `lefthook.yml` заменить блок `test-onboarding` на

```yaml
    test:
      root: app/
      glob: 'app/src/**/*.ts'
      run: npm test
```

В `.github/workflows/ci.yml` строку `      - run: npm run test:onboarding` заменить на `      - run: npm test`.

Run: `npm test`
Expected: PASS, выполняются тесты `pick.test.ts` (glob в кавычках раскрывает сам Node 24).

- [ ] **Step 2: Написать падающий тест**

`app/src/pages/home/model/step.test.ts`:

```ts
import assert from 'node:assert/strict'
import { test } from 'node:test'
import { homeStep } from './step.ts'

test('ни проектов, ни хостов — приветствие', () => {
  assert.equal(homeStep(0, 0), 'welcome')
})

test('хост есть, проектов нет — выбор проекта', () => {
  assert.equal(homeStep(0, 1), 'pick')
  assert.equal(homeStep(0, 3), 'pick')
})

test('проекты есть — на проект, хосты не важны', () => {
  assert.equal(homeStep(2, 0), 'redirect')
  assert.equal(homeStep(1, 3), 'redirect')
})
```

- [ ] **Step 3: Запустить и увидеть падение**

Run: `npm test`
Expected: FAIL — `Cannot find module '.../pages/home/model/step.ts'`.

- [ ] **Step 4: Реализовать**

`app/src/pages/home/model/step.ts`:

```ts
export type HomeStep = 'welcome' | 'pick' | 'redirect'

/** Что показывает `/` (спека, § 2): без проектов и хостов — вход, хост есть — выбор проекта, проекты есть — последний открытый. */
export function homeStep(projects: number, hosts: number): HomeStep {
  if (projects > 0) return 'redirect'
  return hosts > 0 ? 'pick' : 'welcome'
}
```

- [ ] **Step 5: Запустить и увидеть успех**

Run: `npm test && npm run typecheck`
Expected: PASS, 0 ошибок.

- [ ] **Step 6: Коммит**

```bash
git add app/package.json lefthook.yml .github/workflows/ci.yml app/src/pages/home/model
git commit -m "feat(home): homeStep — какой экран показывает / ; общий npm test"
```

---

### Task 2: Строки интерфейса

**Files:**
- Modify: `app/src/shared/i18n/locales/{ru,en,de,es,fr,it,ja,zh}.json`

**Interfaces:**
- Produces: ключи `form.welcome.{title,text,skip}`, `form.connect.{title,github,gitlab,server,manual,trust,back,copyOpen,pickTitle,pickOther}`, `form.sidebar.{accounts,connect}`; новый текст `form.host.add`. Старые ключи удаляют задачи 3 и 5 вместе с их потребителями.

- [ ] **Step 1: Добавить ключи скриптом**

Создать вне репозитория `$TMPDIR/add-locales.mjs` и запустить из `app/`: `node $TMPDIR/add-locales.mjs`. Скрипт переписывает файлы через `JSON.stringify(…, null, 2)`: однострочные объекты (`form.oauth`, `form.settings.source`, `form.settings.themes`) станут многострочными — это ожидаемо.

```js
import { readFileSync, writeFileSync } from 'node:fs'

const T = {
  ru: {
    welcome: { title: 'Добро пожаловать в Longpole', text: 'Войди, чтобы видеть свои пайплайны и находить, что тормозит сборку.', skip: 'Пропустить' },
    connect: { title: 'Подключить аккаунт', github: 'Войти через GitHub', gitlab: 'Войти через GitLab', server: 'Другой сервер…', manual: 'Вставить токен вручную', trust: 'Только чтение · токен хранится в связке ключей', back: '← Назад', copyOpen: 'Скопировать код и открыть {{host}}', pickTitle: 'Выбери проект на {{host}}', pickOther: 'Вставить ссылку или выбрать папку' },
    sidebar: { accounts: 'Аккаунты', connect: 'Подключить' },
    hostAdd: '+ Другой аккаунт…',
  },
  en: {
    welcome: { title: 'Welcome to Longpole', text: 'Sign in to see your pipelines and find what slows your build down.', skip: 'Skip' },
    connect: { title: 'Connect account', github: 'Sign in with GitHub', gitlab: 'Sign in with GitLab', server: 'Other server…', manual: 'Paste a token manually', trust: 'Read-only · token stored in the keychain', back: '← Back', copyOpen: 'Copy code and open {{host}}', pickTitle: 'Pick a project on {{host}}', pickOther: 'Paste a link or pick a folder' },
    sidebar: { accounts: 'Accounts', connect: 'Connect' },
    hostAdd: '+ Another account…',
  },
  de: {
    welcome: { title: 'Willkommen bei Longpole', text: 'Anmelden, um Pipelines zu sehen und herauszufinden, was den Build bremst.', skip: 'Überspringen' },
    connect: { title: 'Konto verbinden', github: 'Mit GitHub anmelden', gitlab: 'Mit GitLab anmelden', server: 'Anderer Server…', manual: 'Token manuell einfügen', trust: 'Nur Lesezugriff · Token im Schlüsselbund gespeichert', back: '← Zurück', copyOpen: 'Code kopieren und {{host}} öffnen', pickTitle: 'Projekt auf {{host}} auswählen', pickOther: 'Link einfügen oder Ordner auswählen' },
    sidebar: { accounts: 'Konten', connect: 'Verbinden' },
    hostAdd: '+ Weiteres Konto…',
  },
  es: {
    welcome: { title: 'Te damos la bienvenida a Longpole', text: 'Inicia sesión para ver tus pipelines y descubrir qué ralentiza la compilación.', skip: 'Omitir' },
    connect: { title: 'Conectar cuenta', github: 'Iniciar sesión con GitHub', gitlab: 'Iniciar sesión con GitLab', server: 'Otro servidor…', manual: 'Pegar un token manualmente', trust: 'Solo lectura · el token se guarda en el llavero', back: '← Atrás', copyOpen: 'Copiar el código y abrir {{host}}', pickTitle: 'Elige un proyecto en {{host}}', pickOther: 'Pega un enlace o elige una carpeta' },
    sidebar: { accounts: 'Cuentas', connect: 'Conectar' },
    hostAdd: '+ Otra cuenta…',
  },
  fr: {
    welcome: { title: 'Bienvenue dans Longpole', text: 'Connectez-vous pour voir vos pipelines et trouver ce qui ralentit le build.', skip: 'Passer' },
    connect: { title: 'Connecter un compte', github: 'Se connecter avec GitHub', gitlab: 'Se connecter avec GitLab', server: 'Autre serveur…', manual: 'Coller un jeton manuellement', trust: 'Lecture seule · jeton stocké dans le trousseau', back: '← Retour', copyOpen: 'Copier le code et ouvrir {{host}}', pickTitle: 'Choisissez un projet sur {{host}}', pickOther: 'Collez un lien ou choisissez un dossier' },
    sidebar: { accounts: 'Comptes', connect: 'Connecter' },
    hostAdd: '+ Autre compte…',
  },
  it: {
    welcome: { title: 'Benvenuto in Longpole', text: 'Accedi per vedere le tue pipeline e scoprire cosa rallenta la build.', skip: 'Salta' },
    connect: { title: 'Collega account', github: 'Accedi con GitHub', gitlab: 'Accedi con GitLab', server: 'Altro server…', manual: 'Incolla un token manualmente', trust: 'Sola lettura · token salvato nel portachiavi', back: '← Indietro', copyOpen: 'Copia il codice e apri {{host}}', pickTitle: 'Scegli un progetto su {{host}}', pickOther: 'Incolla un link o scegli una cartella' },
    sidebar: { accounts: 'Account', connect: 'Collega' },
    hostAdd: '+ Altro account…',
  },
  ja: {
    welcome: { title: 'Longpole へようこそ', text: 'サインインしてパイプラインを表示し、ビルドを遅くしている原因を見つけましょう。', skip: 'スキップ' },
    connect: { title: 'アカウントを接続', github: 'GitHub でサインイン', gitlab: 'GitLab でサインイン', server: 'その他のサーバー…', manual: 'トークンを手動で貼り付け', trust: '読み取り専用 · トークンはキーチェーンに保存', back: '← 戻る', copyOpen: 'コードをコピーして {{host}} を開く', pickTitle: '{{host}} のプロジェクトを選択', pickOther: 'リンクを貼り付けるかフォルダを選択' },
    sidebar: { accounts: 'アカウント', connect: '接続' },
    hostAdd: '+ 別のアカウント…',
  },
  zh: {
    welcome: { title: '欢迎使用 Longpole', text: '登录以查看你的流水线，找出拖慢构建的原因。', skip: '跳过' },
    connect: { title: '连接账户', github: '使用 GitHub 登录', gitlab: '使用 GitLab 登录', server: '其他服务器…', manual: '手动粘贴令牌', trust: '只读 · 令牌保存在钥匙串中', back: '← 返回', copyOpen: '复制代码并打开 {{host}}', pickTitle: '选择 {{host}} 上的项目', pickOther: '粘贴链接或选择文件夹' },
    sidebar: { accounts: '账户', connect: '连接' },
    hostAdd: '+ 其他账户…',
  },
}

for (const [lang, t] of Object.entries(T)) {
  const file = `src/shared/i18n/locales/${lang}.json`
  const json = JSON.parse(readFileSync(file, 'utf8'))
  json.form.welcome = t.welcome
  json.form.connect = t.connect
  Object.assign(json.form.sidebar, t.sidebar)
  json.form.host.add = t.hostAdd
  writeFileSync(file, JSON.stringify(json, null, 2) + '\n')
}
```

- [ ] **Step 2: Проверить**

Run: `npm run check:locales && git diff --stat`
Expected: проверка зелёная; изменены 8 файлов локалей.

- [ ] **Step 3: Коммит**

```bash
git add app/src/shared/i18n/locales
git commit -m "feat(i18n): строки приветствия, «Подключить аккаунт» и блока «Аккаунты»"
```

---

### Task 3: `ConnectAccount`, `OauthWaiting`, стор диалога, логотипы

**Files:**
- Create: `app/src/shared/ui/provider-icon.tsx`
- Create: `app/src/features/manage-token/model/connect.ts`
- Create: `app/src/features/manage-token/ui/OauthWaiting.tsx`
- Create: `app/src/features/manage-token/ui/ConnectAccount.tsx`
- Modify: `app/src/features/manage-token/ui/OauthLogin.tsx` (ветка `if (waiting)`)
- Modify: `app/src/features/manage-token/index.ts`
- Modify: локали — удалить `form.oauth.copy`, `form.oauth.open`

**Interfaces:**
- Consumes: `useOauthLogin(host)` → `{ login(clientId: string | null): Promise<boolean>, cancel(): void, waiting: boolean, code: UserCode | null, error: ApiError | null }` (существует); `TokenForm({ defaultHost, onSaved })` (существует).
- Produces:
  - `ProviderIcon({ provider: Provider | null; className?: string })` из `shared/ui/provider-icon`;
  - `openConnect(): void`, `closeConnect(): void`, `useConnectOpen(): boolean` из `features/manage-token`;
  - `ConnectAccount({ onConnected?: () => void })` из `features/manage-token`;
  - `OauthWaiting({ host: string; code: UserCode | null; onCancel: () => void; className?: string })` — внутренний для слайса.

- [ ] **Step 1: Логотипы**

`app/src/shared/ui/provider-icon.tsx`:

```tsx
import { ServerIcon } from 'lucide-react'
import type { Provider } from '../api'
import { cn } from '../lib/cn'

// брендовых иконок в lucide нет; контуры упрощены до 16×16
const PATHS: Record<Provider, string> = {
  github:
    'M8 0a8 8 0 0 0-2.53 15.59c.4.07.55-.17.55-.38v-1.49c-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.5 7.5 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 8 0Z',
  gitlab:
    'm15.73 6.33-.02-.06L13.53.6a.57.57 0 0 0-1.08.06l-1.47 4.5H5.02L3.55.66A.57.57 0 0 0 2.47.6L.29 6.27l-.02.06a4.05 4.05 0 0 0 1.34 4.68l3.4 2.55 1.69 1.27 1.02.77a.69.69 0 0 0 .83 0l1.02-.77 1.69-1.27 3.42-2.56a4.05 4.05 0 0 0 1.05-4.67Z',
}

/** Логотип провайдера; хост без известного типа — нейтральный сервер. Декоративный: имя хоста — в тексте рядом. */
export function ProviderIcon({ provider, className }: { provider: Provider | null; className?: string }) {
  if (!provider) return <ServerIcon aria-hidden className={cn('size-4', className)} />
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" aria-hidden className={cn('size-4 shrink-0', className)}>
      <path d={PATHS[provider]} />
    </svg>
  )
}
```

- [ ] **Step 2: Стор диалога**

`app/src/features/manage-token/model/connect.ts` — по образцу `features/add-project/model/dialog.ts`:

```ts
import { create } from 'zustand'

// В фиче, а не в виджете диалога: открывают его сайдбар и «Добавить проект», а виджеты не импортируют друг друга
const useConnectStore = create<{ open: boolean }>(() => ({ open: false }))

export const openConnect = () => useConnectStore.setState({ open: true })
export const closeConnect = () => useConnectStore.setState({ open: false })
export const useConnectOpen = () => useConnectStore((s) => s.open)
```

- [ ] **Step 3: `OauthWaiting`**

`app/src/features/manage-token/ui/OauthWaiting.tsx`:

```tsx
import type { UserCode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { Button, buttonVariants } from '../../../shared/ui/button'
import { Giraffe } from '../../../shared/ui/giraffe'
import { GITHUB_APP_INSTALL } from '../model/oauth'

/** Ожидание входа: у GitHub — код и «Скопировать код и открыть», у GitLab браузер открывает ядро. */
export function OauthWaiting({ host, code, onCancel, className }: { host: string; code: UserCode | null; onCancel: () => void; className?: string }) {
  const { t } = useTranslation()
  return (
    <div role="status" className={cn('flex flex-col items-start gap-2', className)}>
      <Giraffe pose="loading" size={48} />
      {code ? (
        <>
          <p className="text-sm">{t('form.oauth.enterCode')}</p>
          <p translate="no" className="font-mono text-2xl tracking-widest select-all">
            {code.userCode}
          </p>
          {/* ссылка, а не кнопка: копируем по клику, а переход уходит в системный браузер через on_navigation */}
          <a href={code.verificationUri} target="_blank" rel="noopener noreferrer" className={buttonVariants()} onClick={() => void navigator.clipboard.writeText(code.userCode)}>
            {t('form.connect.copyOpen', { host })}
          </a>
          {host === 'github.com' && (
            <a href={GITHUB_APP_INSTALL} target="_blank" rel="noopener noreferrer" className="text-xs text-link hover:underline">
              {t('form.oauth.installOrg')}
            </a>
          )}
        </>
      ) : (
        <p className="text-sm">{t('form.oauth.finishInBrowser')}</p>
      )}
      <Button type="button" variant="ghost" onClick={onCancel}>
        {t('form.cancel')}
      </Button>
    </div>
  )
}
```

Проверить, что `UserCode` экспортирован из `shared/api`: `grep -n UserCode app/src/shared/api/index.ts`. `OauthLogin.tsx` уже получает `code` типа `UserCode` из `useOauthLogin`, так что экспорт есть; если нет — добавить `UserCode` в `export type { … }` файла `shared/api/index.ts`.

- [ ] **Step 4: `OauthLogin` — ожидание через `OauthWaiting`**

В `app/src/features/manage-token/ui/OauthLogin.tsx` заменить весь блок `if (waiting) { return (…) }` на

```tsx
  if (waiting) return <OauthWaiting host={host} code={code} onCancel={cancel} />
```

Добавить `import { OauthWaiting } from './OauthWaiting'`; из импорта `../model/oauth` убрать `GITHUB_APP_INSTALL` (остаются `CLOUD_HOSTS`, `registerAppUrl`). `linkClasses` остаётся — им пользуется ссылка «Зарегистрировать».

- [ ] **Step 5: `ConnectAccount`**

`app/src/features/manage-token/ui/ConnectAccount.tsx`:

```tsx
import { useState } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Giraffe } from '../../../shared/ui/giraffe'
import { ProviderIcon } from '../../../shared/ui/provider-icon'
import { useOauthLogin } from '../model/useOauthLogin'
import { OauthWaiting } from './OauthWaiting'
import { TokenForm } from './TokenForm'

/**
 * Вход с первого экрана и из «Подключить аккаунт» (спека, § 5): github.com и gitlab.com в один клик,
 * свой сервер и токен вручную — через `TokenForm`, где есть и OAuth с Client ID, и PAT.
 */
export function ConnectAccount({ onConnected }: { onConnected?: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const [server, setServer] = useState(false)
  const github = useOauthLogin('github.com')
  const gitlab = useOauthLogin('gitlab.com')
  const failure = messageError(github.error ?? gitlab.error)

  if (server) {
    return (
      <div className="flex w-full flex-col gap-2 text-left">
        <Button type="button" variant="ghost" size="sm" className="self-start" onClick={() => setServer(false)}>
          {t('form.connect.back')}
        </Button>
        <TokenForm defaultHost="" onSaved={onConnected} />
      </div>
    )
  }
  if (github.waiting) return <OauthWaiting host="github.com" code={github.code} onCancel={github.cancel} className="items-center text-center" />
  if (gitlab.waiting) return <OauthWaiting host="gitlab.com" code={gitlab.code} onCancel={gitlab.cancel} className="items-center text-center" />

  const start = async (login: typeof github.login) => {
    if (await login(null)) onConnected?.()
  }
  return (
    <div className="flex w-full flex-col gap-2">
      {/* белый на тёмном — 15:1; на оранжевом GitLab было бы ≈2,9:1, поэтому GitLab — outline с оранжевым логотипом */}
      <Button type="button" className="h-8 from-[#24292f] to-[#1d1d1f] text-white dark:from-[#f5f5f7] dark:to-[#e5e5ea] dark:text-[#1d1d1f]" onClick={() => void start(github.login)}>
        <ProviderIcon provider="github" />
        {t('form.connect.github')}
      </Button>
      <Button type="button" variant="outline" className="h-8" onClick={() => void start(gitlab.login)}>
        <ProviderIcon provider="gitlab" className="text-[#e24329]" />
        {t('form.connect.gitlab')}
      </Button>
      <Button type="button" variant="outline" className="h-8" onClick={() => setServer(true)}>
        {t('form.connect.server')}
      </Button>
      <p className="text-xs text-muted-foreground">🔒 {t('form.connect.trust')}</p>
      <button type="button" className="text-xs text-link hover:underline" onClick={() => setServer(true)}>
        {t('form.connect.manual')}
      </button>
      {failure && (
        <p role="alert" className="flex items-start gap-2 text-left text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
    </div>
  )
}
```

Ошибка остаётся под кнопками, повтор — той же кнопкой (`useOauthLogin.login` сбрасывает ошибку при старте). Отмена возвращает к кнопкам без сообщения: при `false` `useOauthLogin` ошибку не ставит.

- [ ] **Step 6: Экспорты**

`app/src/features/manage-token/index.ts` — добавить

```ts
export { closeConnect, openConnect, useConnectOpen } from './model/connect'
export { ConnectAccount } from './ui/ConnectAccount'
```

- [ ] **Step 7: Удалить ключи `form.oauth.copy` и `form.oauth.open`**

Из `app/`:

```bash
node -e "const fs=require('fs'); for (const l of ['ru','en','de','es','fr','it','ja','zh']) { const f='src/shared/i18n/locales/'+l+'.json'; const j=JSON.parse(fs.readFileSync(f,'utf8')); delete j.form.oauth.copy; delete j.form.oauth.open; fs.writeFileSync(f, JSON.stringify(j,null,2)+'\n') }"
grep -rn "form.oauth.copy\|form.oauth.open" src
```

Expected: `grep` ничего не находит.

- [ ] **Step 8: Проверки**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales && npm test`
Expected: всё зелёное.

- [ ] **Step 9: Коммит**

```bash
git add app/src/shared/ui/provider-icon.tsx app/src/features/manage-token app/src/shared/i18n/locales
git commit -m "feat(manage-token): ConnectAccount — вход в один клик для github.com и gitlab.com"
```

---

### Task 4: Виджет `connect-account`, «+ Другой аккаунт…»

**Files:**
- Create: `app/src/widgets/connect-account/ui/PickProject.tsx`
- Create: `app/src/widgets/connect-account/ui/ConnectDialog.tsx`
- Create: `app/src/widgets/connect-account/index.ts`
- Modify: `app/src/features/manage-token/ui/HostSelect.tsx`
- Modify: `app/src/entities/host/model/hosts.ts`, `app/src/entities/host/index.ts`
- Modify: `app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx:97`
- Modify: `app/src/app/form/router.tsx` (оба `<AddProjectDialog />`)

**Interfaces:**
- Consumes: `ConnectAccount`, `closeConnect`, `useConnectOpen`, `openConnect` (задача 3); `useAddProject()` → `{ add: (input: string, options?: { onSuccess?: () => void }) => void, busy: boolean, error }` (это `mutation.mutate`); `ProjectsPanel({ host, onPick })`, `resetSearch()`; `useHostState()` → `{ tokenHost: string | null, saved: HostInfo[], … }`.
- Produces: `PickProject({ host: string; onAdded?: () => void })`, `ConnectDialog()` из `widgets/connect-account`; `HostSelect({ onChange, onAddAccount, children? })`.

- [ ] **Step 1: `PickProject`**

`app/src/widgets/connect-account/ui/PickProject.tsx`:

```tsx
import { useEffect } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Giraffe } from '../../../shared/ui/giraffe'
import { openAddProject, useAddProject } from '../../../features/add-project'
import { ProjectsPanel, resetSearch } from '../../../features/search-projects'

/** Проекты только что подключённого хоста (спека, § 4); выбор добавляет проект и открывает его. */
export function PickProject({ host, onAdded }: { host: string; onAdded?: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { add, busy, error } = useAddProject()
  const failure = messageError(error)
  // строка поиска — общий стор; запрос с прошлого хоста здесь не нужен
  useEffect(() => {
    resetSearch()
  }, [host])
  return (
    <div className="flex w-full flex-col gap-3 text-left">
      <h2 className="text-[15px] font-semibold">{t('form.connect.pickTitle', { host })}</h2>
      <ProjectsPanel host={host} onPick={(project) => !busy && add(`https://${host}/${project.fullPath}`, { onSuccess: () => onAdded?.() })} />
      {failure && (
        <p role="alert" className="flex items-start gap-2 text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
      <button
        type="button"
        className="self-start text-sm text-link hover:underline"
        onClick={() => {
          onAdded?.()
          openAddProject()
        }}
      >
        {t('form.connect.pickOther')}
      </button>
    </div>
  )
}
```

- [ ] **Step 2: `ConnectDialog`**

`app/src/widgets/connect-account/ui/ConnectDialog.tsx`:

```tsx
import { useState } from 'react'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '../../../shared/ui/dialog'
import { closeConnect, ConnectAccount, useConnectOpen } from '../../../features/manage-token'
import { useHostState } from '../../../entities/host'
import { PickProject } from './PickProject'

/** «Подключить аккаунт» из сайдбара и «Добавить проект»: вход, затем выбор проекта на новом хосте. */
export function ConnectDialog() {
  const open = useConnectOpen()
  return (
    <Dialog open={open} onOpenChange={(next) => !next && closeConnect()}>
      {/* тело внутри DialogContent: Radix размонтирует его при закрытии, и следующее открытие начинается с кнопок входа */}
      <DialogContent className="max-w-md">
        <ConnectBody />
      </DialogContent>
    </Dialog>
  )
}

function ConnectBody() {
  const { t } = useTranslation()
  const [connected, setConnected] = useState(false)
  // после входа useOauthLogin/useSaveToken уже вызвали pickHost: tokenHost — только что подключённый хост
  const { tokenHost } = useHostState()
  return (
    <>
      <DialogTitle>{t('form.connect.title')}</DialogTitle>
      <DialogDescription>{t('form.connect.trust')}</DialogDescription>
      {connected && tokenHost ? <PickProject host={tokenHost} onAdded={closeConnect} /> : <ConnectAccount onConnected={() => setConnected(true)} />}
      <Button type="button" variant="ghost" className="self-end" onClick={closeConnect}>
        {t('form.cancel')}
      </Button>
    </>
  )
}
```

`app/src/widgets/connect-account/index.ts`:

```ts
export { ConnectDialog } from './ui/ConnectDialog'
export { PickProject } from './ui/PickProject'
```

- [ ] **Step 3: `HostSelect` открывает вход**

В `app/src/features/manage-token/ui/HostSelect.tsx`:

- сигнатура: `export function HostSelect({ onChange, onAddAccount, children }: { onChange: () => void; onAddAccount: () => void; children?: ReactNode })`;
- импорт из `entities/host`: только `pickHost, useHostState`;
- `<Select value={host ?? ''} onValueChange={…}>`, где обработчик:

```tsx
          onValueChange={(value) => {
            // выбор хоста не меняется: вход открывается поверх, а после него pickHost выберет новый хост
            if (value === ADD_HOST) return onAddAccount()
            pickHost(value)
            onChange()
          }}
```

- `<SelectValue placeholder={t('form.host.label')} />`: без сохранённых хостов (`host === null`) показывает подпись, а пункт «+ Другой аккаунт…» остаётся выбираемым (выбор текущего значения Radix не сообщает, поэтому пункт не может быть значением по умолчанию);
- комментарий над `ADD_HOST`: `// Radix Select не принимает пустое значение у пункта, поэтому «+ Другой аккаунт…» — служебная строка.`

- [ ] **Step 4: Удалить `pickAddHost`**

В `app/src/entities/host/model/hosts.ts`:

```ts
/** `auto` — выбора не было, берём первый сохранённый хост. */
type Picked = { kind: 'auto' } | { kind: 'host'; host: string }
```

удалить строку `export const pickAddHost = …`; в `useHostState`:

```ts
  const host = picked.kind === 'host' ? picked.host : (saved[0]?.host ?? null)
```

докстринг `useHostState`: `/** Текущий хост формы и то, что про него известно. \`host === null\` — сохранённых хостов нет. */`.

В `app/src/entities/host/index.ts` убрать `pickAddHost` из экспорта.

Run: `grep -rn pickAddHost src`
Expected: пусто.

- [ ] **Step 5: Подключить в «Добавить проект» и в роутер**

`app/src/widgets/add-project-dialog/ui/AddProjectDialog.tsx`: в импорт из `features/manage-token` добавить `openConnect` (`closeAddProject` уже импортирован); строку `<HostSelect onChange={resetSearch} />` заменить на

```tsx
          {/* модальные Radix-диалоги рядом спорят за фокус — сначала закрываем свой */}
          <HostSelect
            onChange={resetSearch}
            onAddAccount={() => {
              closeAddProject()
              openConnect()
            }}
          />
```

`app/src/app/form/router.tsx`: `import { ConnectDialog } from '../../widgets/connect-account'` и рядом с каждым из двух `<AddProjectDialog />` добавить `<ConnectDialog />`.

- [ ] **Step 6: Проверки**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales && npm test`
Expected: всё зелёное. Если `lint:fsd` сообщает `fsd/insignificant-slice` для `widgets/connect-account` (пока его использует только роутер; второй потребитель, страница `home`, появится в задаче 5) — не коммитить, перейти к задаче 5 и закоммитить обе задачи одним коммитом в её шаге 6.

- [ ] **Step 7: Коммит**

```bash
git add app/src/widgets/connect-account app/src/features/manage-token app/src/entities/host app/src/widgets/add-project-dialog app/src/app/form/router.tsx
git commit -m "feat: диалог «Подключить аккаунт» и «+ Другой аккаунт…» в «Добавить проект»"
```

---

### Task 5: Экран приветствия и mock-режим

**Files:**
- Modify: `app/src/pages/home/ui/HomePage.tsx`
- Modify: `app/src/app/dev/mock.ts`
- Modify: локали — удалить `form.empty.*`

**Interfaces:**
- Consumes: `homeStep` (задача 1), `ConnectAccount` (задача 3), `PickProject` (задача 4), `useHostState()`, `openAddProject()`.

- [ ] **Step 1: `HomePage`**

Заменить содержимое `app/src/pages/home/ui/HomePage.tsx`:

```tsx
import { Navigate } from '@tanstack/react-router'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Card } from '../../../shared/ui/card'
import { Giraffe } from '../../../shared/ui/giraffe'
import { openAddProject } from '../../../features/add-project'
import { ConnectAccount } from '../../../features/manage-token'
import { PickProject } from '../../../widgets/connect-account'
import { useHostState } from '../../../entities/host'
import { useSavedProjects } from '../../../entities/saved-project'
import { useSettings } from '../../../entities/settings'
import { homeStep } from '../model/step'

/** Без проектов и хостов — вход, хост есть — выбор проекта, проекты есть — последний открытый (или первый). */
export function HomePage() {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { projects, ready, error } = useSavedProjects()
  const { data: settings } = useSettings()
  // ошибка чтения хостов — как будто их нет: сбой связки ключей не должен закрывать вход (спека, § 2)
  const { saved: hosts, tokenHost, ready: hostsReady } = useHostState()
  if (error) {
    return (
      <p role="alert" className="text-sm text-destructive">
        {errorText(error)}
      </p>
    )
  }
  if (!ready || !settings || !hostsReady) return <p className="text-sm text-muted-foreground">{t('form.loading')}</p>
  const step = homeStep(projects.length, hosts.length)
  if (step === 'welcome') {
    return (
      <Card className="m-auto w-full max-w-[340px] items-center gap-3 py-8 text-center">
        <Giraffe size={96} />
        <h1 className="text-xl font-semibold">{t('form.welcome.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('form.welcome.text')}</p>
        <ConnectAccount />
        <button type="button" className="text-sm text-muted-foreground hover:underline" onClick={openAddProject}>
          {t('form.welcome.skip')}
        </button>
      </Card>
    )
  }
  if (step === 'pick') {
    return (
      <Card className="m-auto w-full max-w-[480px]">
        <PickProject host={tokenHost ?? hosts[0].host} />
      </Card>
    )
  }
  const last = settings.lastProject
  const target = projects.find((p) => p.host === last?.host && p.path === last?.path) ?? projects[0]
  return (
    <Navigate to="/project/$host/$" params={{ host: target.host, _splat: target.path }} search={{ ref: undefined }} state={{ name: target.name }} replace />
  )
}
```

`m-auto` центрирует карточку: `main` в `router.tsx` — `flex min-h-full flex-col`. Приветствие — 10 строк JSX, отдельный `WelcomePage.tsx` не нужен.

- [ ] **Step 2: Удалить `form.empty.*`**

Из `app/`:

```bash
node -e "const fs=require('fs'); for (const l of ['ru','en','de','es','fr','it','ja','zh']) { const f='src/shared/i18n/locales/'+l+'.json'; const j=JSON.parse(fs.readFileSync(f,'utf8')); delete j.form.empty; fs.writeFileSync(f, JSON.stringify(j,null,2)+'\n') }"
grep -rn "'form.empty" src
```

Expected: `grep` пусто (`form.history.empty` с шаблоном не совпадает).

- [ ] **Step 3: Mock — режим `?welcome`, код GitHub, новый хост**

В `app/src/app/dev/mock.ts`:

- после `const iso = …` добавить

```ts
// `http://localhost:5173/?welcome#/` — первый запуск: ни хостов, ни проектов
const welcome = new URLSearchParams(location.search).has('welcome')
```

- `const hosts: HostInfo[] = [ … ]` → `const hosts: HostInfo[] = welcome ? [] : [ … ]`;
- `let saved: SavedProject[] = [ … ]` → `let saved: SavedProject[] = welcome ? [] : [ … ]`;
- в импорт типов из `../../shared/api` добавить `type UserCode`;
- команду `oauth_login` заменить на

```ts
  oauth_login: async ({ host, onCode }) => {
    // Channel из @tauri-apps/api: геттер onmessage отдаёт обработчик, переданный в oauthLogin
    if (host === 'github.com') (onCode as { onmessage: (c: UserCode) => void }).onmessage({ userCode: 'WDJB-MJHT', verificationUri: 'https://github.com/login/device' })
    await delay(host === 'github.com' ? 4000 : 1500)
    if (!hosts.some((h) => h.host === host)) hosts.push({ host: String(host), source: 'oauth', provider: host === 'github.com' ? 'github' : 'gitlab' })
    return true
  },
```

- [ ] **Step 4: Проверки**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales && npm test`
Expected: всё зелёное.

- [ ] **Step 5: Ручная проверка в браузере**

Run: `npm run dev` (порт 5173), открыть `http://localhost:5173/?welcome#/`.

Проверить и снять скриншоты (Playwright MCP) в светлой и тёмной теме:

1. приветствие: жираф, заголовок, три кнопки, строка «🔒 Только чтение…», «Вставить токен вручную», «Пропустить»;
2. «Войти через GitHub» → код `WDJB-MJHT`, кнопка «Скопировать код и открыть github.com»; через 4 с — «Выбери проект на github.com» со списком;
3. клик по проекту → экран проекта, проект появился в сайдбаре;
4. перезагрузить `?welcome` → «Войти через GitLab» → «Заверши вход в браузере.» → выбор проекта;
5. «Другой сервер…» → форма хоста и токена; «← Назад» возвращает к кнопкам;
6. «Пропустить» → диалог «Добавить проект» поверх приветствия;
7. «Отмена» во время ожидания → снова кнопки, без ошибки.

Если в п. 2 кода нет (у `Channel` нет геттера `onmessage` в установленной версии `@tauri-apps/api`), проверить код в `npm run tauri dev` на настоящем github.com, а из мока строку с `onmessage` убрать.

- [ ] **Step 6: Коммит**

```bash
git add app/src/pages/home app/src/app/dev/mock.ts app/src/shared/i18n/locales
git commit -m "feat(home): экран приветствия со входом через GitHub/GitLab и выбором проекта"
```

---

### Task 6: Блок «Аккаунты» в сайдбаре

**Files:**
- Modify: `app/src/widgets/projects-sidebar/ui/ProjectsSidebar.tsx`

**Interfaces:**
- Consumes: `useHostState().saved: HostInfo[]` (`useHostState` уже импортирован в файле), `openConnect()` (задача 3), `ProviderIcon` (задача 3).

- [ ] **Step 1: Разметка**

В `ProjectsSidebar.tsx`:

- импорты: `Link` — к существующему импорту из `@tanstack/react-router`; `import { PlusIcon } from 'lucide-react'`; `import { ProviderIcon } from '../../../shared/ui/provider-icon'`; `import { cn } from '../../../shared/lib/cn'`; `import { openConnect } from '../../../features/manage-token'`;
- `const { host: currentHost } = useHostState()` → `const { host: currentHost, saved: hosts } = useHostState()`;
- у `Section` добавить проп `className?: string`: `function Section({ title, action, className, children }: { title: string; action?: ReactNode; className?: string; children: ReactNode })` и `<section className={cn('flex flex-col gap-2', className)}>`;
- после секции «Недавние», последним ребёнком `<aside>`:

```tsx
      {/* прижат к низу: аккаунты всегда на виду, второй подключается отсюда (спека, § 6) */}
      <Section title={t('form.sidebar.accounts')} className="mt-auto">
        <ul className="flex flex-col gap-0.5">
          {hosts.map((h) => (
            <li key={h.host}>
              <Link to="/settings" className="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-accent">
                <ProviderIcon provider={h.provider} className="text-muted-foreground" />
                <span translate="no" className="min-w-0 flex-1 truncate">
                  {h.host}
                </span>
                <span className="text-[11px] text-muted-foreground">{t(`form.settings.source.${h.source}`)}</span>
              </Link>
            </li>
          ))}
          <li>
            <button type="button" className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-sm text-link hover:bg-accent" onClick={openConnect}>
              <PlusIcon aria-hidden className="size-4" />
              {t('form.sidebar.connect')}
            </button>
          </li>
        </ul>
      </Section>
```

- [ ] **Step 2: Проверки**

Run: `npm run typecheck && npm run lint:fsd && npm run check:locales && npm test`
Expected: всё зелёное.

- [ ] **Step 3: Ручная проверка**

`npm run dev`, `http://localhost:5173/#/` (без `?welcome`, три хоста из моков):

1. внизу сайдбара «Аккаунты»: три хоста с логотипами и источником, «+ Подключить»;
2. клик по хосту → настройки;
3. «+ Подключить» → диалог «Подключить аккаунт» → GitLab → выбор проекта → проект открыт, диалог закрыт;
4. «+ Добавить» (Мои проекты) → «Найти в списке проектов хоста» → в выборе хоста «+ Другой аккаунт…» → «Добавить проект» закрылся, открылся «Подключить аккаунт»;
5. скриншоты сайдбара в светлой и тёмной теме; длинный хост обрезается многоточием.

- [ ] **Step 4: Коммит**

```bash
git add app/src/widgets/projects-sidebar
git commit -m "feat(sidebar): блок «Аккаунты» с «+ Подключить»"
```

---

## Покрытие спеки

- § 2 → задачи 1, 5; § 3 → 5; § 4 → 4, 5; § 5 → 3; § 6 → 4 (диалог, «Добавить проект»), 6 (сайдбар); § 7 → 2, 3, 5; § 8 → 1, 5, 6.
