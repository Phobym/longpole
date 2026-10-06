import type { Theme } from '../../../shared/api'
import { useTranslation } from '../../../shared/i18n'
import { applyTheme } from '../../../shared/lib/theme'
import { Button } from '../../../shared/ui/button'
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
    <div role="group" aria-label={t('form.settings.theme')} className="flex gap-0.5">
      {THEMES.map((theme) => (
        <Button key={theme} type="button" variant={current === theme ? 'secondary' : 'ghost'} size="sm" aria-pressed={current === theme} onClick={() => void choose(theme)}>
          {t(`form.settings.themes.${theme}`)}
        </Button>
      ))}
    </div>
  )
}
