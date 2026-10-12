/** Системные диалоги Desktop через плагин Tauri. */
import { ask as askDialog, message, open } from '@tauri-apps/plugin-dialog'

export const ask = (text: string, okLabel: string, cancelLabel: string) => askDialog(text, { kind: 'warning', okLabel, cancelLabel })

export const alert = async (text: string) => {
  await message(text, { kind: 'error' })
}

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
