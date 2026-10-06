import type { Theme } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { applyTheme } from '../../../shared/lib/theme'
import { Segmented } from '../../../shared/ui/segmented'
import { useSettings, useUpdateSettings } from '../../../entities/settings'

const THEMES: Theme[] = ['system', 'light', 'dark']

/** Тема применяется сразу и сохраняется в settings.json. */
export function ThemeSwitch() {
  const { t } = useTranslation()
  const { data } = useSettings()
  const update = useUpdateSettings()
  const current = data?.theme ?? 'system'
  const choose = async (theme: Theme) => {
    applyTheme(theme)
    await update({ theme })
  }
  return (
    <Segmented
      label={t('form.settings.theme')}
      value={current}
      options={THEMES.map((theme) => ({ value: theme, label: t(`form.settings.themes.${theme}`) }))}
      onChange={(theme) => void choose(theme)}
    />
  )
}
