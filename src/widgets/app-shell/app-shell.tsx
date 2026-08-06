import { useQuery } from '@tanstack/react-query'
import { open } from '@tauri-apps/plugin-dialog'
import { Group, Panel, Separator } from 'react-resizable-panels'
import { toast } from 'sonner'
import { useOpenTab } from '@entities/layout/layout.query'
import { activeProjectQueryOptions, projectListQueryOptions, useActivateProject, useOpenProject } from '@entities/project/project.query'
import { IS_MAC } from '@shared/constants/platform'
import { TitleBar } from '@features/window/title-bar'
import { WelcomeScreen } from '@features/welcome/welcome-screen'
import { AppSidebar } from '@widgets/app-sidebar/app-sidebar'
import { EditorArea } from '@widgets/editor-area/editor-area'
import { ExplorerContainer } from '@widgets/explorer/explorer-container'

const SETTINGS_TAB_TITLE = '설정'

export const AppShell = () => {
    const { data: projects = [], isPending } = useQuery(projectListQueryOptions())
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { mutate: openProject } = useOpenProject()
    const { mutate: activateProject } = useActivateProject()
    const { mutate: openTab } = useOpenTab(activeProjectId)

    const handleOpenProject = async () => {
        const selected = await open({ directory: true, multiple: false })
        if (typeof selected !== 'string') return
        openProject(selected, { onError: (error) => toast.error(error.message) })
    }

    const handleOpenSettings = () => {
        if (!activeProjectId) return toast.info('먼저 프로젝트를 여세요')
        openTab(
            { projectId: activeProjectId, kind: { kind: 'settings' }, title: SETTINGS_TAB_TITLE, target: null, preview: false },
            { onError: (error) => toast.error(error.message) },
        )
    }

    if (isPending) return <div className='bg-app-background h-full w-full' />

    return (
        <div className='bg-app-background text-app-foreground flex h-full w-full flex-col'>
            {IS_MAC && <TitleBar />}
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
                                <Separator className='bg-app-border hover:bg-ring w-1 cursor-col-resize' />
                                <Panel id='editor' minSize='30%'>
                                    <EditorArea projectId={activeProjectId} />
                                </Panel>
                            </Group>
                        ) : (
                            <span className='text-app-sidebar-icon-default m-auto'>프로젝트를 선택하세요</span>
                        )}
                    </main>
                </div>
            )}
        </div>
    )
}
