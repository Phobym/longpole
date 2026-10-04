import { create } from 'zustand'

type State = { url: string; focusRequested: boolean; setUrl: (url: string) => void; fill: (url: string) => void; focused: () => void }

const useLinkStore = create<State>((set) => ({
  url: '',
  focusRequested: false,
  setUrl: (url) => set({ url }),
  fill: (url) => set({ url, focusRequested: true }),
  focused: () => set({ focusRequested: false }),
}))

/** Запись истории режима «ссылка»: подставляет URL и просит поле забрать фокус. */
export const fillLink = (url: string) => useLinkStore.getState().fill(url)

export const useLink = () => useLinkStore()
