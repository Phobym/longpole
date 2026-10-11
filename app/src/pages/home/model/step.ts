export type HomeStep = 'welcome' | 'pick' | 'redirect'

/** Что показывает `/` (спека, § 2): без проектов и хостов — вход, хост есть — выбор проекта, проекты есть — последний открытый. */
export function homeStep(projects: number, hosts: number): HomeStep {
  if (projects > 0) return 'redirect'
  return hosts > 0 ? 'pick' : 'welcome'
}
