/** Видимый интервал оси, мс от создания корневого пайплайна. */
export type View = readonly [start: number, end: number]

/** Позиция момента на дорожке, % ширины. */
export const pos = (ms: number, [a, b]: View) => ((ms - a) / (b - a)) * 100

/** Ось не уходит левее создания пайплайна: отсчёт всегда с нуля. */
export const clampView = ([a, b]: View): View => (a < 0 ? [0, b - a] : [a, b])

const STEPS = [1e3, 5e3, 1e4, 3e4, 6e4, 12e4, 3e5, 6e5, 9e5, 18e5, 36e5, 72e5]

/** Шаг делений из ряда 1s … 2h: наименьший, при котором на интервале не больше `maxTicks` делений. */
export const tickStep = (span: number, maxTicks: number) => STEPS.find((x) => span / x <= maxTicks) ?? STEPS[STEPS.length - 1]
