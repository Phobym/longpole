import type { InfiniteData, UseInfiniteQueryResult } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { apiError } from '../api/unwrap'
import type { Page } from '../api/schema/Page'
import { apiErrorText, useTranslation } from '../i18n'
import { Button } from './button'

type Props<T> = { query: UseInfiniteQueryResult<InfiniteData<Page<T>>>; children: (item: T) => ReactNode }

/** Список постраничного запроса: «Загрузка…», «Ничего не найдено», ошибка над списком и «Показать ещё». */
export function PagedList<T>({ query, children }: Props<T>) {
  const { t } = useTranslation()
  const items = query.data?.pages.flatMap((page) => page.items) ?? []
  const failure = query.isError ? apiError(query.error) : null
  const status = query.isPending || query.isFetchingNextPage ? t('form.loading') : !query.isError && items.length === 0 ? t('form.nothing') : ''
  return (
    <div className="flex flex-col gap-2">
      <p aria-live="polite" className="text-sm text-muted-foreground empty:hidden">
        {status}
      </p>
      {query.isError && (
        <p role="alert" className="text-sm text-destructive">
          {failure ? apiErrorText(t, failure) : String(query.error)}
        </p>
      )}
      <ul className="flex flex-col gap-1.5">{items.map(children)}</ul>
      {query.hasNextPage && !query.isError && (
        <Button type="button" variant="ghost" className="self-start" onClick={() => void query.fetchNextPage()}>
          {t('form.more')}
        </Button>
      )}
    </div>
  )
}
