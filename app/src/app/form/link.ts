/**
 * Ссылка на отчёт (спека поставки, § 8): путь пайплайна, MR, run или PR у провайдера, где домен провайдера заменён
 * доменом Longpole. Вместо пути проекта — и числовой id. В домене всегда есть точка: страницы приложения так не начинаются.
 */
const LINK = /^\/([^/]+\.[^/]+)\/(?:\d+|[^/]+(?:\/[^/]+)+?)\/(?:-\/(?:pipelines|merge_requests)|actions\/runs|pull)\/\d+(?:\/.*)?$/

/** Хост и ссылка провайдера, которую разбирает ядро; не ссылка — `null`. */
export const reportLink = (pathname: string): { host: string; url: string } | null => {
  const host = LINK.exec(pathname)?.[1]
  return host ? { host, url: `https://${pathname.slice(1)}` } : null
}
