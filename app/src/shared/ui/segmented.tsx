import { cn } from '../lib/cn'

type Option<T extends string> = { value: T; label: string; lang?: string }

type Props<T extends string> = {
  /** Доступное имя группы. */
  label: string
  value: T
  options: Option<T>[]
  onChange: (value: T) => void
  /** `sm` — 22px (форма), `xs` — 20px (шапка отчёта). */
  size?: 'sm' | 'xs'
}

/** Сегментированный переключатель как в macOS: подложка `secondary`, активный пункт приподнят тенью. */
export function Segmented<T extends string>({ label, value, options, onChange, size = 'sm' }: Props<T>) {
  return (
    <div role="group" aria-label={label} className="inline-flex rounded-md bg-secondary p-px">
      {options.map((option) => {
        const active = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            lang={option.lang}
            aria-pressed={active}
            onClick={() => onChange(option.value)}
            className={cn(
              'rounded-[5px] px-2.5 font-medium whitespace-nowrap focus-ring',
              size === 'sm' ? 'h-[22px] text-xs' : 'h-5 text-[11px]',
              active ? 'bg-card shadow-[0_0.5px_2px_rgb(0_0_0/0.2)]' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {option.label}
          </button>
        )
      })}
    </div>
  )
}
