export { isAggNode, type ReportNode } from './node'
export type { AggNode } from './schema/AggNode'
export type { Excess } from './schema/Excess'
export type { Hotspot } from './schema/Hotspot'
export type { Kind } from './schema/Kind'
export type { Locale } from './schema/Locale'
export type { Report } from './schema/Report'
export type { SingleNode } from './schema/SingleNode'
export type { Stability } from './schema/Stability'
export type { Tree } from './schema/Tree'

/** Эталонные отчёты для dev-сервера; грузятся лениво, в сборку отчёта не попадают. */
export const fixtures = {
  single: () => import('./fixtures/single.json'),
  aggregate: () => import('./fixtures/aggregate.json'),
}
