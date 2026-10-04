import { createHashHistory, createRootRoute, createRoute, createRouter, Outlet, useRouter } from '@tanstack/react-router'
import { useEffect } from 'react'
import { HistorySidebar } from '../../widgets/history-sidebar'
import { LanguageSwitch } from '../../widgets/language-switch'
import { ProjectPage } from '../../pages/project'
import { ProjectsPage } from '../../pages/projects'

function Layout() {
  const router = useRouter()
  // Esc на экране «Проект» — назад к «Проектам». Один обработчик на всё приложение; открытый Popover/Select
  // гасит Esc сам (`defaultPrevented`), поэтому первый Esc закрывает его, второй возвращает к «Проектам».
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || e.defaultPrevented) return
      if (router.state.location.pathname.startsWith('/project/')) void router.navigate({ to: '/' })
    }
    document.addEventListener('keydown', onKeyDown)
    return () => document.removeEventListener('keydown', onKeyDown)
  }, [router])

  return (
    <div className="flex h-screen">
      <HistorySidebar />
      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-4xl flex-col gap-3 p-6">
          <div className="flex justify-end">
            <LanguageSwitch />
          </div>
          <Outlet />
        </div>
      </main>
    </div>
  )
}

const rootRoute = createRootRoute({ component: Layout })

const projectsRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: ProjectsPage })

// Хост — параметр, путь проекта (`group/sub/project`) — splat, ветка — `?ref=`.
const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/project/$host/$',
  validateSearch: (search: Record<string, unknown>): { ref?: string } => ({
    ref: typeof search.ref === 'string' && search.ref !== '' ? search.ref : undefined,
  }),
  component: () => {
    const { host, _splat } = projectRoute.useParams()
    const { ref } = projectRoute.useSearch()
    // key: другой проект — другая страница, прогресс, ошибки и подсказки веток прежнего не переносятся
    return <ProjectPage key={`${host}/${_splat}`} host={host} fullPath={_splat ?? ''} branch={ref} />
  },
})

export const router = createRouter({
  routeTree: rootRoute.addChildren([projectsRoute, projectRoute]),
  history: createHashHistory(),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
