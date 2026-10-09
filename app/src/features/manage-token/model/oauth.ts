import type { Provider } from '../../../shared/api'

/** Хосты, где client_id — наш и вводить его не нужно. */
export const CLOUD_HOSTS = ['github.com', 'gitlab.com']

/** Без установки GitHub App на организацию её репозитории не видны. */
export const GITHUB_APP_INSTALL = 'https://github.com/apps/longpole/installations/new'

/** Страница регистрации приложения на self-hosted: там пользователь берёт client_id. */
export const registerAppUrl = (host: string, provider: Provider) =>
  provider === 'github' ? `https://${host}/settings/apps/new` : `https://${host}/-/user_settings/applications`
