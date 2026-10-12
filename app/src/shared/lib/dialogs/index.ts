import i18next from 'i18next'
import type { ApiError } from '../../api'
import { DELIVERY } from '../../config/delivery'
import { errorText } from '../../i18n'
import * as desktop from './desktop'
import * as selfhosted from './selfhosted'

// как в `shared/api`: ветка другого варианта поставки вырезается при сборке
const native: typeof desktop = DELIVERY === 'desktop' ? desktop : selfhosted

export const { pickFolder } = native

/** Подтверждение; если диалог не открылся, это отказ: лучше ничего не удалить, чем удалить молча. */
export async function confirm(text: string, okLabel: string): Promise<boolean> {
  try {
    return await native.ask(text, okLabel, i18next.t('form.cancel'))
  } catch (e) {
    console.error(e)
    return false
  }
}

export async function showError(error: ApiError): Promise<void> {
  try {
    await native.alert(errorText(i18next.t, error))
  } catch (e) {
    console.error(e) // показать нечем: хотя бы в консоль
  }
}
