import { useId } from 'react'
import type { Workflow } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { labelClasses } from '../../../shared/ui/input'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../../shared/ui/select'

type Props = { workflows: readonly Workflow[]; value: string; onChange: (file: string) => void }

/** Workflow GitHub: последние запуски и агрегат строятся по одному workflow. */
export function WorkflowSelect({ workflows, value, onChange }: Props) {
  const { t } = useTranslation()
  const id = useId()
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className={labelClasses}>
        {t('form.workflow.label')}
      </label>
      <Select value={value} onValueChange={onChange}>
        <SelectTrigger id={id} translate="no">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {workflows.map((w) => (
            <SelectItem key={w.file} value={w.file}>
              {w.name} · {w.file}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}
