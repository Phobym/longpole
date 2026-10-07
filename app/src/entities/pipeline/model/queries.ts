import { useInfiniteQuery } from '@tanstack/react-query'
import { pipelines, unwrap } from '../../../shared/api'

export const usePipelines = (host: string, project: string, ref: string | undefined) =>
  useInfiniteQuery({
    queryKey: ['pipelines', host, project, ref ?? null],
    queryFn: ({ pageParam }) => unwrap(pipelines(host, project, ref ?? null, null, pageParam)),
    initialPageParam: null as string | null,
    staleTime: 0, // список последних пайплайнов устаревает сразу: при каждом открытии проекта берём свежий
    getNextPageParam: (last) => last.next,
  })

/** Ссылка, по которой ядро строит отчёт одного пайплайна. */
export const pipelineUrl = (host: string, project: string, id: string) => `https://${host}/${project}/-/pipelines/${id}`
