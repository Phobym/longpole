export { isAggNode, type ReportNode } from './node'
export type { Excess } from './schema/Excess'
export type { Locale } from './schema/Locale'
export type { Report } from './schema/Report'
export type { Stability } from './schema/Stability'
export type { Tree } from './schema/Tree'

/** Эталонные отчёты для dev-сервера; грузятся лениво, в сборку отчёта не попадают. */
export const fixtures = {
  single: () => import('./fixtures/single.json'),
  aggregate: () => import('./fixtures/aggregate.json'),
}
