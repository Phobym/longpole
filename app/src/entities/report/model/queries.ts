import { useQuery } from '@tanstack/react-query'
import { report, unwrap } from '../../../shared/api'

/** Кеш отчётов живёт в Rust: здесь не держим копию дольше, чем экран смонтирован. */
export const useReportById = (id: number) =>
  useQuery({ queryKey: ['report', id], queryFn: () => unwrap(report(id)), staleTime: Infinity, gcTime: 0 })
