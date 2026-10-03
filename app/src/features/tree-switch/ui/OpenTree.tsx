import type { ComponentProps } from 'react'
import { useReportView } from '../../../entities/report'

/** Кнопка открывает пайплайн из агрегата (`trees[index]`): группы свёрнуты, выделения нет, ось на весь пайплайн. */
export function OpenTree({ index, onClick, ...props }: { index: number } & ComponentProps<'button'>) {
  const { dispatch } = useReportView()
  return (
    <button
      type="button"
      {...props}
      onClick={(e) => {
        onClick?.(e)
        dispatch({ type: 'open', tree: index })
      }}
    />
  )
}
