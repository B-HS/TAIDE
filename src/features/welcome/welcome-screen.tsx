import type { FC } from 'react'
import { FolderOpen } from 'lucide-react'
import { MOD_KEY_LABEL } from '@shared/constants/platform'
import { Button } from '@shared/ui/button'

type RecentProject = {
    id: string
    name: string
    root: string
}

type WelcomeScreenProps = {
    recentProjects: RecentProject[]
    onOpenProject: () => void
    onSelectRecent: (id: string) => void
}

export const WelcomeScreen: FC<WelcomeScreenProps> = ({ recentProjects, onOpenProject, onSelectRecent }) => (
    <div className='bg-app-background flex h-full w-full items-center justify-center'>
        <div className='flex w-96 flex-col gap-6'>
            <div className='flex flex-col gap-1'>
                <span className='text-2xl font-semibold tracking-tight'>TAIDE</span>
                <span className='text-app-sidebar-icon-default'>폴더를 열어 프로젝트를 시작하세요</span>
            </div>

            <Button onClick={onOpenProject} className='w-fit'>
                <FolderOpen />
                폴더 열기
                <kbd className='ml-2 text-xs opacity-70'>{MOD_KEY_LABEL}O</kbd>
            </Button>

            {recentProjects.length > 0 && (
                <div className='flex flex-col gap-2'>
                    <span className='text-panel-section-header text-xs font-medium'>최근 항목</span>
                    {recentProjects.map((project) => (
                        <button
                            key={project.id}
                            type='button'
                            onClick={() => onSelectRecent(project.id)}
                            className='hover:bg-app-sidebar-item-hover flex flex-col items-start rounded-md px-2 py-1 text-left'>
                            <span>{project.name}</span>
                            <span className='text-app-sidebar-icon-default truncate text-xs'>{project.root}</span>
                        </button>
                    ))}
                </div>
            )}
        </div>
    </div>
)
