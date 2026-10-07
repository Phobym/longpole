import { useQuery } from '@tanstack/react-query'
import { hostProvider, unwrap, type Provider } from '../../../shared/api'

const GITHUB_COM = 'github.com'

/** Тип хоста для подсказки о токене; github.com известен сразу, остальные — из кэша ядра или пробы. Пока неизвестен — `undefined`. */
export function useHostProvider(host: string): Provider | undefined {
  const { data } = useQuery({
    queryKey: ['hostProvider', host],
    queryFn: () => unwrap(hostProvider(host)),
    enabled: host !== GITHUB_COM,
    staleTime: Infinity,
  })
  return host === GITHUB_COM ? 'github' : data
}
