import { create } from 'zustand'

type State = { url: string; focusLinkToken: number; handledToken: number }

const useLinkStore = create<State>(() => ({ url: '', focusLinkToken: 0, handledToken: 0 }))

export const useLinkUrl = () => useLinkStore((s) => s.url)
export const setLinkUrl = (url: string) => useLinkStore.setState({ url })

/** Запись истории режима «ссылка»: подставляет URL и просит поле забрать фокус. */
export const fillLink = (url: string) => useLinkStore.setState((s) => ({ url, focusLinkToken: s.focusLinkToken + 1 }))

/** Есть ли невыполненная просьба о фокусе; после возврата на экран старая не повторяется. */
export const useFocusRequested = () => useLinkStore((s) => s.focusLinkToken > s.handledToken)
export const markFocusHandled = () => useLinkStore.setState((s) => ({ handledToken: s.focusLinkToken }))
