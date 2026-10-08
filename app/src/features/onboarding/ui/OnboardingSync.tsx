import { useEffect, useRef } from 'react'
import { useSettings, useUpdateSettings } from '../../../entities/settings'
import { hydrate, setPersist } from '../model/store'

/**
 * Настройки → стор подсказок (один раз) и запись изменений стора обратно. Только в форме: в сохранённом отчёте его нет, и подсказки выключены.
 * После первой загрузки источник правды — стор: повторный `hydrate` устаревшим ответом отменил бы только что сделанное в нём.
 */
export function OnboardingSync() {
  const { data } = useSettings()
  const update = useUpdateSettings()
  const hydrated = useRef(false)
  useEffect(() => setPersist((patch) => void update(patch)), [update])
  useEffect(() => {
    if (!data || hydrated.current) return
    hydrated.current = true
    hydrate(data.tips, data.seenTips)
  }, [data])
  return null
}
