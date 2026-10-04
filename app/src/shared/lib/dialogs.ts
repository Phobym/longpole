import { ask, message } from '@tauri-apps/plugin-dialog'
import i18next from 'i18next'
import type { ApiError } from '../api'
import { errorText } from '../i18n'

/** Подтверждение; если диалог не открылся, это отказ: лучше ничего не удалить, чем удалить молча. */
export async function confirm(text: string, okLabel: string): Promise<boolean> {
  try {
    return await ask(text, { kind: 'warning', okLabel, cancelLabel: i18next.t('form.cancel') })
  } catch (e) {
    console.error(e)
    return false
  }
}

export async function showError(error: ApiError): Promise<void> {
  try {
    await message(errorText(i18next.t, error), { kind: 'error' })
  } catch (e) {
    console.error(e) // показать нечем: хотя бы в консоль
  }
}
