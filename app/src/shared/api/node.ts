import type { AggNode } from './schema/AggNode'
import type { SingleNode } from './schema/SingleNode'

/** Узел дерева отчёта: одиночного пайплайна или агрегата. */
export type ReportNode = SingleNode | AggNode

export const isAggNode = (node: ReportNode): node is AggNode => 'stats' in node
