import { useQuery } from '@tanstack/react-query'
import { history, unwrap } from '../../../shared/api'

export const historyKey = ['history'] as const

export const useHistory = () => useQuery({ queryKey: historyKey, queryFn: () => unwrap(history()) })
