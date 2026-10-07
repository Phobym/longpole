import { useInfiniteQuery } from '@tanstack/react-query'
import { pipelines, unwrap } from '../../../shared/api'

export const usePipelines = (host: string, project: string, ref: string | undefined, workflow: string | undefined) =>
  useInfiniteQuery({
    queryKey: ['pipelines', host, project, ref ?? null, workflow ?? null],
    queryFn: ({ pageParam }) => unwrap(pipelines(host, project, ref ?? null, workflow ?? null, pageParam)),
    initialPageParam: null as string | null,
    staleTime: 0, // список последних пайплайнов устаревает сразу: при каждом открытии проекта берём свежий
    getNextPageParam: (last) => last.next,
  })
