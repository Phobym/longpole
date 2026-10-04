import { PagedList } from '../../../shared/ui/paged-list'
import { ProjectSearch, useDebouncedSearch } from '../../../features/search-projects'
import { ProjectRow, useOpenProject, useProjects } from '../../../entities/project'

export function ProjectsPanel({ host }: { host: string }) {
  const query = useProjects(host, useDebouncedSearch())
  const openProject = useOpenProject()
  return (
    <div className="flex flex-col gap-3">
      <ProjectSearch />
      <PagedList query={query}>
        {(project) => (
          <ProjectRow
            key={project.fullPath}
            project={project}
            onOpen={() => void openProject({ host, project: project.fullPath, branch: project.defaultBranch ?? undefined, name: project.name })}
          />
        )}
      </PagedList>
    </div>
  )
}
