import { useEffect, useState } from 'react'

/** Пауза после последнего ввода перед запросом поиска. */
export const SEARCH_DEBOUNCE = 300

/** Значение, которое подхватывает `value` через `ms` после последнего изменения; начальное — сразу. */
export function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms)
    return () => clearTimeout(timer)
  }, [value, ms])
  return debounced
}
