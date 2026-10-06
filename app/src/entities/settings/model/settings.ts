import { queryOptions, useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { getSettings, setSettings, unwrap, type SettingsPatch } from '../../../shared/api'
import { showError } from '../../../shared/lib/dialogs'

export const settingsQuery = queryOptions({ queryKey: ['settings'], queryFn: () => unwrap(getSettings()), staleTime: Infinity })

export const useSettings = () => useQuery(settingsQuery)

/** Частичное обновление; ответ ядра — полные настройки, они сразу в кеше. Стабильна: годится в зависимости эффектов. */
export function useUpdateSettings() {
  const queryClient = useQueryClient()
  return useCallback(
    async (patch: SettingsPatch) => {
      const result = await setSettings(patch)
      if (result.ok) queryClient.setQueryData(settingsQuery.queryKey, result.value)
      else await showError(result.error)
      return result.ok
    },
    [queryClient],
  )
}
