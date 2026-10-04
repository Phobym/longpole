import { ask, message } from '@tauri-apps/plugin-dialog'
import type { TFunction } from 'i18next'
import type { ApiError } from '../api'
import { apiErrorText } from '../i18n'

/** Подтверждение; если диалог не открылся, это отказ: лучше ничего не удалить, чем удалить молча. */
export async function confirm(t: TFunction, text: string, okLabel: string): Promise<boolean> {
  try {
    return await ask(text, { kind: 'warning', okLabel, cancelLabel: t('form.cancel') })
  } catch (e) {
    console.error(e)
    return false
  }
}

export async function showError(t: TFunction, error: ApiError): Promise<void> {
  try {
    await message(apiErrorText(t, error), { kind: 'error' })
  } catch (e) {
    console.error(e) // показать нечем: хотя бы в консоль
  }
}
