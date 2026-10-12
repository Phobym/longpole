import { useEffect } from 'react'
import { messageError } from '../../../shared/api'
import { useErrorText, useTranslation } from '../../../shared/i18n'
import { Giraffe } from '../../../shared/ui/giraffe'
import { openAddProject, useAddProject } from '../../../features/add-project'
import { ProjectsPanel, resetSearch } from '../../../features/search-projects'

/** Проекты только что подключённого хоста (спека, § 4); выбор добавляет проект и открывает его. */
export function PickProject({ host, onAdded }: { host: string; onAdded?: () => void }) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const { add, busy, error } = useAddProject()
  const failure = messageError(error)
  // строка поиска — общий стор; запрос с прошлого хоста здесь не нужен
  useEffect(() => {
    resetSearch()
  }, [host])
  return (
    <div className="flex w-full flex-col gap-3 text-left">
      <h2 className="text-[15px] font-semibold">{t('form.connect.pickTitle', { host })}</h2>
      <ProjectsPanel host={host} onPick={(project) => !busy && add(`https://${host}/${project.fullPath}`, { onSuccess: () => onAdded?.() })} />
      {failure && (
        <p role="alert" className="flex items-start gap-2 text-sm text-destructive">
          <Giraffe pose="oops" size={40} />
          {errorText(failure)}
        </p>
      )}
      <button
        type="button"
        className="self-start text-sm text-link hover:underline"
        onClick={() => {
          onAdded?.()
          openAddProject()
        }}
      >
        {t('form.connect.pickOther')}
      </button>
    </div>
  )
}
