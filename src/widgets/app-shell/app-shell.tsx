import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { useTranslation } from 'react-i18next'
import { Group, Panel } from 'react-resizable-panels'
import { toast } from 'sonner'
import { useOpenTab } from '@entities/layout/layout.query'
import { activeProjectQueryOptions, projectListQueryOptions, useActivateProject, useOpenProject } from '@entities/project/project.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { PaneSeparator } from '@features/split/pane-separator'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { IS_MAC } from '@shared/constants/platform'
import { WelcomeScreen } from '@features/welcome/welcome-screen'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'
import { StatusBarContent } from '@widgets/window-chrome/status-bar-content'
import { TitleBarContent } from '@widgets/window-chrome/title-bar-content'

export const AppShell = () => {
    const { t } = useTranslation()
    const { data: projects = [], isPending } = useQuery(projectListQueryOptions())
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: openTab } = useOpenTab(activeProjectId)

    const handleOpenProject = async () => {
        const selected = await open({ directory: true, multiple: false })
        if (typeof selected !== 'string') return
        openProject(selected, { onError: (error) => toast.error(error.message) })
    }

    const handleOpenSettings = () => {
        if (!activeProjectId) return toast.info(t('app.openProjectFirst'))
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: t('settings.title'), target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    if (isPending) return <div className='bg-app-background h-full w-full' />

    return (
        <div className='bg-app-background text-app-foreground flex h-full w-full flex-col'>
            {IS_MAC && <TitleBarContent />}
            {projects.length === 0 ? (
                <div className='min-h-0 flex-1'>
                    <WelcomeScreen recentProjects={[]} onOpenProject={() => void handleOpenProject()} onSelectRecent={(id) => activateProject(id)} />
                </div>
            ) : (
                <div className='flex min-h-0 flex-1'>
                    <AppSidebar activeProjectId={activeProjectId} onOpenSettings={handleOpenSettings} />
                    <main className='flex min-w-0 flex-1'>
                        {activeProjectId ? (
                            <Group orientation='horizontal' className='min-h-0 min-w-0 flex-1'>
                                <Panel id='explorer' defaultSize='240px' minSize='180px' maxSize='40%' collapsible collapsedSize={0}>
                                    <ExplorerContainer projectId={activeProjectId} />
                                </Panel>
                                <PaneSeparator orientation='horizontal' thickness={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS} />
                                <Panel id='editor' minSize='30%'>
                                    <EditorArea projectId={activeProjectId} />
                                </Panel>
                            </Group>
                        ) : (
                            <span className='text-app-sidebar-icon-default m-auto'>{t('app.selectProject')}</span>
                        )}
                    </main>
                </div>
            )}
            <StatusBarContent />
        </div>
    )
}
