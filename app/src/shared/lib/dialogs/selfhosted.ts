/** Диалоги браузера: подписи кнопок у них не задаются. */
export const ask = async (text: string) => window.confirm(text)

export const alert = async (text: string) => window.alert(text)

/** Папок на сервере нет: `capabilities.addByFolder` выключен, кнопку выбора интерфейс прячет по нему (#80). */
export const pickFolder = async (): Promise<string | null> => null
