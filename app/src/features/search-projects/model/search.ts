import { create } from 'zustand'
import { SEARCH_DEBOUNCE, useDebounced } from '../../../shared/lib/useDebounced'

const useSearchStore = create<{ text: string; setText: (text: string) => void }>((set) => ({
  text: '',
  setText: (text) => set({ text }),
}))

export const resetSearch = () => useSearchStore.getState().setText('')

/** Текст запроса с паузой: после «← Проекты» он уже в сторе, поэтому поиск и список из кеша Query на месте сразу. */
export function useDebouncedSearch() {
  const text = useSearchStore((s) => s.text)
  const debounced = useDebounced(text, SEARCH_DEBOUNCE)
  return text === '' ? '' : debounced // сброс (смена хоста) не должен ждать паузу и слать запрос со старым текстом
}

export const useSearchText = () => useSearchStore((s) => s.text)
export const setSearchText = (text: string) => useSearchStore.getState().setText(text)
