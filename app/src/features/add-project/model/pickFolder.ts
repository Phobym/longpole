import { open } from '@tauri-apps/plugin-dialog'

/** Системный выбор папки; отмена и сбой диалога — `null`. */
export async function pickFolder(): Promise<string | null> {
  try {
    const chosen = await open({ directory: true, multiple: false })
    return typeof chosen === 'string' ? chosen : null
  } catch (e) {
    console.error(e)
    return null
  }
}
