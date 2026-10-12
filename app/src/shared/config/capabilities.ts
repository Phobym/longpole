import { DELIVERY } from './delivery'

const desktop = DELIVERY === 'desktop'

/** Что умеет вариант поставки; в Selfhosted всё `false`, кроме `reportCache`. */
export const capabilities = {
  /** общий кэш отчётов на сервере: в шапке отчёта «Перестроить» мимо него */
  reportCache: !desktop,
  /** свои хосты и токены */
  manageHosts: desktop,
  /** добавление проекта по папке */
  addByFolder: desktop,
  /** ⌘S через системный диалог */
  nativeSave: desktop,
  /** вход без ухода со страницы */
  oauthLoopback: desktop,
  /** маршруты в hash: окно грузит файл, сервера, который отдал бы `index.html` на любой путь, нет — и ссылки на отчёт тоже */
  hashRouting: desktop,
} as const
