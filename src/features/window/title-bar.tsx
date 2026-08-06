import type { FC } from 'react'
import { GitBranch } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { TITLE_BAR_TRAFFIC_LIGHT_INSET_PX } from '@shared/constants/window-chrome'

type TitleBarProps = {
    tabTitle: string | null
    projectName: string | null
    branch: string | null
}

export const TitleBar: FC<TitleBarProps> = ({ tabTitle, projectName, branch }) => {
    const { t } = useTranslation()

    const primaryText = tabTitle ?? projectName
    const secondaryText = tabTitle && projectName ? projectName : null

    return (
        <div data-tauri-drag-region className='bg-app-background text-app-foreground flex h-7 shrink-0 items-center text-xs select-none'>
            <div data-tauri-drag-region style={{ width: TITLE_BAR_TRAFFIC_LIGHT_INSET_PX }} className='shrink-0' />
            <div data-tauri-drag-region className='flex min-w-0 flex-1 items-center justify-center gap-1.5 px-2'>
                {primaryText && (
                    <span data-tauri-drag-region className='min-w-0 truncate'>
                        {primaryText}
                    </span>
                )}
                {secondaryText && (
                    <span data-tauri-drag-region className='hidden min-w-0 shrink items-center gap-1.5 truncate opacity-70 md:inline-flex'>
                        <span data-tauri-drag-region>{t('window.titleSeparator')}</span>
                        <span data-tauri-drag-region className='truncate'>
                            {secondaryText}
                        </span>
                    </span>
                )}
                {branch && (
                    <span data-tauri-drag-region className='text-app-sidebar-icon-default hidden shrink-0 items-center gap-1 lg:inline-flex'>
                        <GitBranch data-tauri-drag-region className='size-3' />
                        <span data-tauri-drag-region className='max-w-24 truncate'>
                            {branch}
                        </span>
                    </span>
                )}
            </div>
            <div data-tauri-drag-region style={{ width: TITLE_BAR_TRAFFIC_LIGHT_INSET_PX }} className='shrink-0' />
        </div>
    )
}
