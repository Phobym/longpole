import { useReportView } from '../../../entities/report'
import { useRevealNode } from '../../../features/node-selection'
import { useTranslation } from '../../../shared/i18n'
import { cn } from '../../../shared/lib/cn'
import { Section } from './Section'

/** «Связи»: от кого выбранная ждала (↑) и кто ждёт её (↓); клик выделяет и показывает джобу. */
export function Relations() {
  const { tree, rel } = useReportView()
  const { t } = useTranslation()
  const reveal = useRevealNode()
  const links = [...[...rel.up].map((id) => ({ id, dir: 'up' as const })), ...[...rel.down].map((id) => ({ id, dir: 'down' as const }))]
  if (!links.length) return null
  return (
    <Section title={t('report.panel.relations')}>
      <div className="flex flex-wrap gap-1.5">
        {links.map(({ id, dir }) => {
          const name = tree.nodes[id]?.name ?? id
          const what = t(dir === 'up' ? 'report.panel.relationUp' : 'report.panel.relationDown')
          return (
            <button
              key={`${dir}:${id}`}
              type="button"
              translate="no"
              aria-label={`${name} — ${what}`}
              title={what}
              className={cn('cursor-pointer rounded-md border border-l-[3px] bg-card px-2 py-0.5 text-left [overflow-wrap:anywhere] hover:bg-accent', dir === 'up' ? 'border-l-dep-up' : 'border-l-dep-down')}
              onClick={() => reveal(id)}
            >
              {dir === 'up' ? '↑' : '↓'} {name}
            </button>
          )
        })}
      </div>
    </Section>
  )
}
