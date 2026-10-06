import type { Project } from '../../../shared/api'
import { PagedList } from '../../../shared/ui/paged-list'
import { ProjectRow, useProjects } from '../../../entities/project'
import { useDebouncedSearch } from '../model/search'
import { ProjectSearch } from './ProjectSearch'

export function ProjectsPanel({ host, onPick }: { host: string; onPick: (project: Project) => void }) {
  const query = useProjects(host, useDebouncedSearch())
  return (
    <div className="flex flex-col gap-3">
      <ProjectSearch />
      <PagedList query={query}>{(project) => <ProjectRow key={project.fullPath} project={project} onOpen={() => onPick(project)} />}</PagedList>
    </div>
  )
}
