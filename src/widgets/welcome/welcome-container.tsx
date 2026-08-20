import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { toast } from 'sonner'
import type { Project, ProjectId } from '@shared/api/bindings'
import { useOpenTab } from '@entities/layout/layout.query'
import { projectListQueryOptions, recentProjectsQueryOptions, useActivateProject, useOpenProject } from '@entities/project/project.query'
import { WelcomeScreen } from '@features/welcome/welcome-screen'

const fileNameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1)

type WelcomeContainerProps = {
    /** The project this Welcome surface is scoped to — `null` for the zero-projects full-screen
     *  case (`AppShell`), a real id for the `welcome` tab inside an already-open project
     *  (`PaneNodeView`). Drives whether file-open is available and, when it is, which project's
     *  root a picked file resolves and opens against. */
    projectId: ProjectId | null
}

/**
 * Owns every data fetch/mutation the Welcome surface needs and hands the pure
 * `WelcomeScreen` its props — shared verbatim by `AppShell`'s zero-projects screen and
 * `PaneNodeView`'s `welcome` tab (contract §1.2 "적용 면 2곳 통일") so the two never drift.
 */
export const WelcomeContainer: FC<WelcomeContainerProps> = ({ projectId }) => {
    const { data: openProjects = [] } = useQuery(projectListQueryOptions())
    const { data: recentProjects = [] } = useQuery(recentProjectsQueryOptions())
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: openTab } = useOpenTab(projectId)

    const handleOpenFolder = async () => {
        const selected = await open({ directory: true, multiple: false })
        if (typeof selected !== 'string') return
        openProject(selected, { onError: (error) => toast.error(error.message) })
    }

    const handleOpenFile = async () => {
        if (!projectId) return
        const scopedProject = openProjects.find((project) => project.id === projectId)
        const selected = await open({ multiple: false, defaultPath: scopedProject?.root })
        if (typeof selected !== 'string') return
        openTab(
            { projectId, kind: { kind: 'file', path: selected }, title: fileNameOf(selected), target: null, preview: true },
            { onError: (error) => toast.error(error.message) },
        )
    }

    const handleSelectRecent = (project: Project) => {
        const isAlreadyOpen = openProjects.some((openProjectEntry) => openProjectEntry.id === project.id)
        if (isAlreadyOpen) {
            activateProject(project.id, { onError: (error) => toast.error(error.message) })
            return
        }
        openProject(project.root, { onError: (error) => toast.error(error.message) })
    }

    return (
        <WelcomeScreen
            recentProjects={recentProjects}
            onOpenFolder={() => void handleOpenFolder()}
            canOpenFile={projectId !== null}
            onOpenFile={() => void handleOpenFile()}
            onSelectRecent={handleSelectRecent}
        />
    )
}
