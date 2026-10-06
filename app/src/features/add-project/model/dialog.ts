import { create } from 'zustand'

const useDialogStore = create<{ open: boolean }>(() => ({ open: false }))

export const openAddProject = () => useDialogStore.setState({ open: true })
export const closeAddProject = () => useDialogStore.setState({ open: false })
export const useAddProjectOpen = () => useDialogStore((s) => s.open)
