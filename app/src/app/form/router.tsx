import { createHashHistory, createRootRoute, createRoute, createRouter, Outlet, useMatches, useNavigate } from '@tanstack/react-router'
import { useEffect } from 'react'
import { useGoBack } from '../../shared/lib/useGoBack'
import { openAddProject } from '../../features/add-project'
import { HomePage } from '../../pages/home'
import { ProjectPage } from '../../pages/project'
import { SettingsPage } from '../../pages/settings'
import { ReportRoute } from './report-route'
import { AddProjectDialog } from '../../widgets/add-project-dialog'
import { AppHeader } from '../../widgets/app-header'
import { ProjectsSidebar } from '../../widgets/projects-sidebar'

const isMac = navigator.platform.startsWith('Mac')
const mod = (e: KeyboardEvent) => (isMac ? e.metaKey : e.ctrlKey)

function Layout() {
  const navigate = useNavigate()
  const goBack = useGoBack()
  const onReport = useMatches({ select: (matches) => matches.some((m) => m.routeId === '/report/$id') })
  const onSettings = useMatches({ select: (matches) => matches.some((m) => m.routeId === '/settings') })

  // Один обработчик клавиш на приложение. Открытый оверлей (Popover/Select) и выделенная строка отчёта гасят Esc сами (`defaultPrevented`):
  // первый Esc закрывает оверлей или снимает выделение, второй уводит с отчёта или настроек.
  // Слушаем `window`, а не `document`: он срабатывает после document-слушателей независимо от порядка их регистрации.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return
      if (e.key === 'Escape' && (onReport || onSettings)) {
        goBack()
      } else if (mod(e) && e.shiftKey && e.key.toLowerCase() === 'n') {
        e.preventDefault()
        openAddProject()
      } else if (mod(e) && e.key === ',') {
        e.preventDefault()
        void navigate({ to: '/settings' })
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [goBack, navigate, onReport, onSettings])

  // отчёту нужна вся ширина: без сайдбара и шапки
  if (onReport) {
    return (
      <main className="h-screen overflow-y-auto">
        <Outlet />
        <AddProjectDialog />
      </main>
    )
  }
  return (
    <div className="flex h-screen">
      <ProjectsSidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <AppHeader />
        {/* контент на «хроме» окна, страница — одна inset-grouped карточка на всю ширину */}
        <main className="flex-1 overflow-y-auto bg-chrome">
          <div className="flex min-h-full flex-col gap-3 px-5 py-4">
            <Outlet />
          </div>
          <AddProjectDialog />
        </main>
      </div>
    </div>
  )
}

const rootRoute = createRootRoute({ component: Layout })

const homeRoute = createRoute({ getParentRoute: () => rootRoute, path: '/', component: HomePage })

// Хост — параметр, путь проекта (`group/sub/project`) — splat, ветка — `?ref=`, workflow GitHub — `?workflow=`.
const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/project/$host/$',
  validateSearch: (search: Record<string, unknown>): { ref?: string; workflow?: string } => ({
    ref: typeof search.ref === 'string' && search.ref !== '' ? search.ref : undefined,
    workflow: typeof search.workflow === 'string' && search.workflow !== '' ? search.workflow : undefined,
  }),
  component: () => {
    const { host, _splat } = projectRoute.useParams()
    const { ref } = projectRoute.useSearch()
    // key: другой проект — другая страница, прогресс, ошибки и подсказки веток прежнего не переносятся
    return <ProjectPage key={`${host}/${_splat}`} host={host} project={_splat ?? ''} branch={ref} />
  },
})

export const reportRoute = createRoute({ getParentRoute: () => rootRoute, path: '/report/$id', component: ReportRoute })
const settingsRoute = createRoute({ getParentRoute: () => rootRoute, path: '/settings', component: SettingsPage })

export const router = createRouter({
  routeTree: rootRoute.addChildren([homeRoute, projectRoute, reportRoute, settingsRoute]),
  history: createHashHistory(),
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}
