import type { FC } from 'react'
import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { Project, ProjectId } from '@shared/api/bindings'
import type { KeymapActionId, KeymapEntry } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap/keymap'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { currentWindowFocusedPane } from '@shared/lib/pane-tree'
import { isWithinRoot } from '@shared/lib/path-root'
import { layoutQueryOptions, useOpenFileTab } from '@entities/layout/layout.query'
import {
    projectListQueryOptions,
    recentProjectsQueryOptions,
    useActivateProject,
    useOpenFolderDialog,
    useOpenProject,
} from '@entities/project/project.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { WelcomeScreen } from '@features/welcome/welcome-screen'

const RECENT_PROJECT_DISPLAY_LIMIT = 8

/** A representative slice of `APP_KEYMAP` surfaced on the Welcome screen's shortcuts card — kept
 *  short (navigation/panel/save) rather than exhaustive; the full catalog lives in the keybindings
 *  editor (`settings.keymapOpenEditor`). Ids only, so the label/shortcut text is always derived
 *  from the *effective* keymap (`APP_KEYMAP` with `settings.keymapOverrides` applied, same as
 *  `useGlobalKeymap`'s actual dispatch and the command palette's display) — never hand-duplicated,
 *  and never stale after the user rebinds or unbinds one of these actions. */
const WELCOME_KEYMAP_HIGHLIGHT_IDS: KeymapActionId[] = ['quick-open', 'command-palette', 'search', 'toggle-terminal', 'toggle-sidebar', 'save']

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
    const { t } = useTranslation()
    const { data: openProjects = [] } = useQuery(projectListQueryOptions())
    const { data: recentProjects = [], isError: isRecentProjectsError } = useQuery(recentProjectsQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const openFileTab = useOpenFileTab()
    const handleOpenFolder = useOpenFolderDialog()

    const effectiveKeymap = applyKeymapOverrides(APP_KEYMAP, parseKeymapOverrides(settings?.keymapOverrides ?? null))
    const shortcuts = WELCOME_KEYMAP_HIGHLIGHT_IDS.map((id) => effectiveKeymap.find((entry) => entry.id === id)).filter(
        (entry): entry is KeymapEntry => entry != null,
    )
    const recentProjectsExcludingSelf = recentProjects.filter((project) => project.id !== projectId)
    const displayedRecentProjects = recentProjectsExcludingSelf.slice(0, RECENT_PROJECT_DISPLAY_LIMIT)

    const handleOpenFile = async () => {
        if (!projectId) return
        const scopedProject = openProjects.find((project) => project.id === projectId)
        const selected = await open({ multiple: false, defaultPath: scopedProject?.root })
        if (typeof selected !== 'string') return
        if (scopedProject && !isWithinRoot(selected, scopedProject.root)) {
            toast.error(t('app.openFileOutsideRoot'))
            return
        }
        openFileTab({ projectId, path: selected, target: currentWindowFocusedPane(layout), preview: false })
    }

    const handleSelectRecent = (project: Project) => {
        const isAlreadyOpen = openProjects.some((openProjectEntry) => openProjectEntry.id === project.id)
        if (isAlreadyOpen) {
            activateProject(project.id, { onError: (error) => toast.error(describeIpcError(error)) })
            return
        }
        openProject(project.root, { onError: (error) => toast.error(describeIpcError(error)) })
    }

    return (
        <WelcomeScreen
            recentProjects={displayedRecentProjects}
            recentProjectsUnavailable={isRecentProjectsError}
            shortcuts={shortcuts}
            onOpenFolder={handleOpenFolder}
            canOpenFile={projectId !== null}
            onOpenFile={() => void handleOpenFile()}
            onSelectRecent={handleSelectRecent}
        />
    )
}
