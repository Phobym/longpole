import { type Form } from '../../../shared/api'

// `Form` у ядра — сырая форма: все поля строками, лишнее ядро отбрасывает.
const blank: Form = { mode: 'link', host: '', url: '', project: '', ref: '', source: '', last: '', statuses: [] }

export const linkForm = (url: string): Form => ({ ...blank, mode: 'link', url })

export const aggregateForm = (f: Pick<Form, 'host' | 'project' | 'ref' | 'last' | 'statuses'>): Form => ({
  ...blank,
  mode: 'aggregate',
  ...f,
})
