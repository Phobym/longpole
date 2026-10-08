import { useEffect } from 'react'
import { startVisit } from './store'

/** Экран смонтирован — новый визит: можно одну `enter`-подсказку. Эффект родителя идёт после эффектов детей-якорей, поэтому выбор — уже по готовым. */
export function useOnboardingVisit() {
  useEffect(startVisit, [])
}
