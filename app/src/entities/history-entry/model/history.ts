import { queryOptions, useQuery } from '@tanstack/react-query'
import { apiError, history, unwrap } from '../../../shared/api'

export const historyQuery = queryOptions({ queryKey: ['history'], queryFn: () => unwrap(history()) })

export function useHistory() {
  const { data, error } = useQuery(historyQuery)
  return { entries: data ?? [], error: apiError(error) }
}
