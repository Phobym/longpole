import { useQuery } from '@tanstack/react-query'
import { report, unwrap } from '../../../shared/api'

export const isReportId = (id: number) => Number.isInteger(id)

/** Кеш отчётов живёт в Rust: здесь не держим копию дольше, чем экран смонтирован. */
export const useReportById = (id: number) =>
  useQuery({ queryKey: ['report', id], queryFn: () => unwrap(report(id)), staleTime: Infinity, gcTime: 0, enabled: isReportId(id) })
