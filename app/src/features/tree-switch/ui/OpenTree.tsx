import type { ComponentProps } from 'react'
import { useReportView } from '../../../entities/report'

/** Кнопка открывает пайплайн из агрегата (`trees[index]`): группы свёрнуты, выделения нет, ось на весь пайплайн. */
export function OpenTree({ index, ...props }: { index: number } & Omit<ComponentProps<'button'>, 'onClick'>) {
  const { dispatch } = useReportView()
  return <button type="button" {...props} onClick={() => dispatch({ type: 'open', tree: index })} />
}
