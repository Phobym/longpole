/** Диалоги браузера: подписи кнопок у них не задаются. */
export const ask = async (text: string, _okLabel: string, _cancelLabel: string) => window.confirm(text)

export const alert = async (text: string) => window.alert(text)

/** Папок на сервере нет: `capabilities.addByFolder` выключен, кнопки выбора нет. */
export const pickFolder = async (): Promise<string | null> => null
