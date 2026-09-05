import type { Project, ProjectId } from '@shared/api/bindings'
import { isWithinRoot } from '@shared/lib/path-root'

type ResolveExternalOpenTargetInput = { path: string; projects: Project[]; activeProjectId: ProjectId | null }

/**
 * Picks the project a CLI-opened file (`taide <file>`, Claude Code's Ctrl+G temp file) becomes a
 * tab of: the most specific open project whose root contains it, else the active project (or the
 * first open one when nothing is active). A Ctrl+G temp file lives under the OS tmpdir and so
 * belongs to no root — the backend lets exactly that file through its boundary
 * (`root_guard::resolve_owning_project_or_cli_opened`), and `isOutsideProjectRoot` tells the editor
 * to keep LSP, format-on-save and the hot-exit mirror off it. `null` only when no project is open
 * at all, the one situation where "open a project first" is the right answer.
 */
export const resolveExternalOpenTarget = ({ path, projects, activeProjectId }: ResolveExternalOpenTargetInput) => {
    const owningProject = projects
        .filter((project) => isWithinRoot(path, project.root))
        .reduce<Project | null>((best, project) => (best && best.root.length >= project.root.length ? best : project), null)
    if (owningProject) return { projectId: owningProject.id, isOutsideProjectRoot: false }

    const activeProject = projects.find((project) => project.id === activeProjectId) ?? projects[0] ?? null
    return activeProject ? { projectId: activeProject.id, isOutsideProjectRoot: true } : null
}
