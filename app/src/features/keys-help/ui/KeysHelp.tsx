import { Trans } from 'react-i18next'
import { useTranslation } from '../../../shared/i18n'
import { Button } from '../../../shared/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '../../../shared/ui/popover'

const KEYS = ['pan', 'zoom', 'rows', 'fold', 'stage', 'job'] as const
const COMPONENTS = { k: <kbd className="font-[inherit] font-semibold" /> }

/** Кнопка `?` и подсказка по управлению: закрывается повторным кликом, кликом вне и Esc. */
export function KeysHelp() {
  const { t } = useTranslation()
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="outline" size="icon" className="size-6 rounded-full text-muted-foreground" aria-label={t('report.help')}>
          ?
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-[min(30rem,90vw)] rounded-[10px] px-3.5 py-2.5 text-[12.5px]">
        <ul className="m-0 list-none p-0">
          {KEYS.map((key) => (
            <li key={key} className="py-0.5">
              <Trans i18nKey={`report.keys.${key}`} components={COMPONENTS} />
            </li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  )
}
