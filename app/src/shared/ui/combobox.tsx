import { Command as CommandPrimitive } from 'cmdk'
import { useRef, useState } from 'react'
import { CommandItem, CommandList } from './command'
import { inputClasses } from './input'
import { Popover, PopoverAnchor, PopoverContent } from './popover'

type Props = {
  /** Доступное имя поля: cmdk сам назначает полю `id` и `aria-labelledby`, поэтому `<label for>` к нему не привязать. */
  label: string
  value: string
  options: string[]
  /** Каждое изменение текста. */
  onChange: (text: string) => void
  /** Выбор подсказки, Enter или уход фокуса: текст не обязан быть из подсказок. */
  onCommit: (text: string) => void
}

/** Поле со свободным текстом и подсказками (Command + Popover); фокус всё время остаётся в поле. */
export function Combobox({ label, value, options, onChange, onCommit }: Props) {
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState('')
  // cmdk сам подсвечивает первую подсказку; Enter берёт её, только если до неё дошли стрелками
  const navigated = useRef(false)
  const input = useRef<HTMLInputElement>(null)

  const shown = open && options.length > 0

  const commit = (text: string) => {
    setOpen(false)
    onCommit(text)
  }

  return (
    <CommandPrimitive label={label} shouldFilter={false} value={active} onValueChange={setActive}>
      <Popover open={shown} onOpenChange={setOpen}>
        <PopoverAnchor asChild>
          <div>
            <CommandPrimitive.Input
              ref={input}
              className={inputClasses}
              value={value}
              onValueChange={(text) => {
                navigated.current = false
                setOpen(true)
                onChange(text)
              }}
              onFocus={() => setOpen(true)}
              onBlur={() => commit(value)}
              onKeyDown={(e) => {
                if (e.key === 'ArrowDown' || e.key === 'ArrowUp') navigated.current = true
                // без открытого списка (после Esc) стрелки ничего не выбирали: Enter всегда берёт набранный текст
                if (e.key === 'Enter' && (!shown || !navigated.current)) {
                  e.stopPropagation()
                  commit(value)
                }
              }}
            />
          </div>
        </PopoverAnchor>
        <PopoverContent
          className="w-(--radix-popover-trigger-width) p-0"
          align="start"
          onOpenAutoFocus={(e) => e.preventDefault()}
          onCloseAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => {
            if (e.target === input.current) e.preventDefault()
          }}
          // клик по подсказке не должен уводить фокус из поля: иначе blur закоммитит набранный текст
          onMouseDown={(e) => e.preventDefault()}
        >
          <CommandList>
            {options.map((option) => (
              <CommandItem key={option} value={option} onSelect={() => commit(option)}>
                {option}
              </CommandItem>
            ))}
          </CommandList>
        </PopoverContent>
      </Popover>
    </CommandPrimitive>
  )
}
