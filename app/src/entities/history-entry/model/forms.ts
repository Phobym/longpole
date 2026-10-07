import type { Form, Status } from '../../../shared/api'

// `Form` у ядра — сырая форма: все поля строками, лишнее ядро отбрасывает.
const blank: Form = { mode: 'link', host: '', url: '', project: '', ref: '', source: '', last: '', statuses: [], workflow: '' }

export const linkForm = (url: string): Form => ({ ...blank, mode: 'link', url })

type Aggregate = { host: string; project: string; ref: string; workflow: string; last: string; statuses: readonly Status[] | null }

/** `statuses === null` — «любой»: в форме ядра это `ANY`. */
export const aggregateForm = ({ statuses, ...rest }: Aggregate): Form => ({
  ...blank,
  mode: 'aggregate',
  ...rest,
  statuses: statuses ? [...statuses] : ['ANY'],
})
