/**
 * Вариант поставки, задаётся при сборке (`VITE_DELIVERY`). Читают только `shared/api`, `shared/lib/dialogs`
 * и `capabilities.ts` — остальной код смотрит на `capabilities` (правило `longpole/delivery-import` в steiger).
 */
export const DELIVERY: 'desktop' | 'selfhosted' = import.meta.env.VITE_DELIVERY === 'selfhosted' ? 'selfhosted' : 'desktop'
