/** Видимый интервал оси, мс от создания корневого пайплайна. */
export type View = readonly [start: number, end: number]

/** Позиция момента на дорожке, % ширины. */
export const pos = (ms: number, [a, b]: View) => ((ms - a) / (b - a)) * 100

/** Ось не уходит левее создания пайплайна: отсчёт всегда с нуля. */
export const clampView = ([a, b]: View): View => (a < 0 ? [0, b - a] : [a, b])
