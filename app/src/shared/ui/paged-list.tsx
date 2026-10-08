import type { InfiniteData, UseInfiniteQueryResult } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { apiError } from '../api/unwrap'
import type { Page } from '../api/schema/Page'
import { useErrorText, useTranslation } from '../i18n'
import { Button } from './button'

type Props<T> = {
  query: UseInfiniteQueryResult<InfiniteData<Page<T>>>
  children: (item: T) => ReactNode
  /** вместо «Ничего не найдено», когда список пуст */
  empty?: ReactNode
}

/** Список постраничного запроса: «Загрузка…», «Ничего не найдено» (или `empty`), ошибка над списком и «Показать ещё». */
export function PagedList<T>({ query, children, empty }: Props<T>) {
  const { t } = useTranslation()
  const errorText = useErrorText()
  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const failure = apiError(query.error)
  const nothing = !query.isPending && !query.isFetchingNextPage && !query.isError && items.length === 0
  const status = query.isPending || query.isFetchingNextPage ? t('form.loading') : nothing ? t('form.nothing') : ''
  return (
    <div className="flex flex-col gap-2">
      {nothing && empty ? (
        empty
      ) : (
        <p aria-live="polite" className="text-sm text-muted-foreground empty:hidden">
          {status}
        </p>
      )}
      {query.isError && (
        <p role="alert" className="text-sm text-destructive">
          {failure && errorText(failure)}
        </p>
      )}
      {/* рамка и zebra — у контейнера, строки без своих рамок (спека, § 4); пустой список не рисует пустую рамку */}
      <ul className="flex flex-col overflow-hidden rounded-lg border empty:hidden">{items.map(children)}</ul>
      {/* после ошибки первой страницы данных нет и `hasNextPage` ложно; после ошибки следующей кнопка нужна для повтора */}
      {query.hasNextPage && (
        <Button type="button" variant="ghost" className="self-start" disabled={query.isFetchingNextPage} onClick={() => void query.fetchNextPage()}>
          {t('form.more')}
        </Button>
      )}
    </div>
  )
}
