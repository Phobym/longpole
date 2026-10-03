import type { ReportNode } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'

const WIDTH = 'w-[34px] flex-none'

/** Стрелка `▸`/`▾` с числом детей; у строки без детей — пустое место того же размера. */
export function GroupToggle({ node, collapsed, onToggle }: { node: ReportNode; collapsed: boolean; onToggle: (id: string) => void }) {
  const { t } = useTranslation()
  if (!node.children.length) return <span className={WIDTH} />
  return (
    <button
      type="button"
      className={`${WIDTH} cursor-pointer text-left text-xs`}
      aria-expanded={!collapsed}
      aria-label={t(collapsed ? 'report.expand' : 'report.collapse', { name: node.name })}
      // клик по стрелке не выделяет строку
      onClick={(e) => {
        e.stopPropagation()
        onToggle(node.id)
      }}
    >
      {collapsed ? '▸' : '▾'}
      <span className="ml-0.5 text-muted-foreground">{node.children.length}</span>
    </button>
  )
}
