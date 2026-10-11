import { create } from 'zustand'

// В фиче, а не в виджете диалога: открывают его сайдбар и «Добавить проект», а виджеты не импортируют друг друга
const useConnectStore = create<{ open: boolean }>(() => ({ open: false }))

export const openConnect = () => useConnectStore.setState({ open: true })
export const closeConnect = () => useConnectStore.setState({ open: false })
export const useConnectOpen = () => useConnectStore((s) => s.open)
