import type { FC } from 'react'
import { Clock, FileText, FolderOpen, Keyboard, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { Project } from '@shared/api/bindings'
import type { KeymapEntry } from '@shared/lib/keymap'
import { formatKeymapShortcut } from '@shared/lib/keymap'
import { Button } from '@shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@shared/ui/card'

type WelcomeScreenProps = {
    recentProjects: Project[]
    recentProjectsUnavailable: boolean
    /** The highlighted subset of the *effective* keymap (`APP_KEYMAP` with the user's
     *  `settings.keymapOverrides` applied) to surface as shortcut hints — computed by
     *  `WelcomeContainer` so this component never reads `APP_KEYMAP` directly and never shows a
     *  binding the user has since rebound or unbound. */
    shortcuts: KeymapEntry[]
    onOpenFolder: () => void
    /** Whether a file-open dialog has a project root to resolve against — false on the very first
     *  launch (no project open anywhere yet, the zero-projects screen always passes `projectId:
     *  null`), true only inside a `welcome` tab of an already-open project. */
    canOpenFile: boolean
    onOpenFile: () => void
    onSelectRecent: (project: Project) => void
}

export const WelcomeScreen: FC<WelcomeScreenProps> = ({
    recentProjects,
    recentProjectsUnavailable,
    shortcuts,
    onOpenFolder,
    canOpenFile,
    onOpenFile,
    onSelectRecent,
}) => {
    const { t } = useTranslation()

    return (
        <div className='bg-app-background flex h-full w-full justify-center overflow-y-auto'>
            <div className='my-auto flex w-96 flex-col gap-6 py-10'>
                <div className='flex flex-col gap-1'>
                    <span className='text-2xl font-semibold tracking-tight'>TAIDE</span>
                    <span className='text-app-sidebar-icon-default'>{t('app.openFolderHint')}</span>
                </div>

                <div className='flex flex-col gap-2'>
                    <div className='flex flex-wrap items-center gap-2'>
                        <Button onClick={onOpenFolder} className='w-fit'>
                            <FolderOpen />
                            {t('app.openFolder')}
                        </Button>
                        <Button onClick={onOpenFile} disabled={!canOpenFile} variant='outline' className='w-fit'>
                            <FileText />
                            {t('app.openFile')}
                        </Button>
                    </div>
                    {!canOpenFile && <span className='text-app-sidebar-icon-default text-xs'>{t('app.openFileHint')}</span>}
                </div>

                {recentProjectsUnavailable ? (
                    <span className='text-app-sidebar-icon-default text-xs'>{t('app.recentProjectsUnavailable')}</span>
                ) : (
                    recentProjects.length > 0 && (
                        <div className='flex flex-col gap-2'>
                            <h2 className='text-panel-section-header flex items-center gap-1.5 text-xs font-medium'>
                                <Clock className='size-3.5' />
                                {t('app.recentItems')}
                            </h2>
                            <ul className='flex flex-col'>
                                {recentProjects.map((project) => (
                                    <li key={project.id}>
                                        <button
                                            type='button'
                                            aria-disabled={project.rootMissing}
                                            onClick={() => !project.rootMissing && onSelectRecent(project)}
                                            className='hover:bg-app-sidebar-item-hover flex w-full flex-col items-start rounded-md px-2 py-1 text-left aria-disabled:cursor-not-allowed aria-disabled:opacity-50'>
                                            <span>{project.name}</span>
                                            <span className='text-app-sidebar-icon-default flex w-full min-w-0 items-center gap-1 text-xs'>
                                                {project.rootMissing && (
                                                    <span className='flex shrink-0 items-center gap-1'>
                                                        <TriangleAlert className='size-3 shrink-0' />
                                                        {t('app.recentProjectRootMissing')}
                                                    </span>
                                                )}
                                                <span className='truncate' title={project.root}>
                                                    {project.root}
                                                </span>
                                            </span>
                                        </button>
                                    </li>
                                ))}
                            </ul>
                        </div>
                    )
                )}

                {shortcuts.length > 0 && (
                    <Card className='py-4'>
                        <CardHeader className='px-4'>
                            <CardTitle className='flex items-center gap-1.5 text-xs'>
                                <Keyboard className='size-3.5' />
                                {t('app.keyboardShortcutsTitle')}
                            </CardTitle>
                        </CardHeader>
                        <CardContent className='grid grid-cols-2 gap-x-4 gap-y-1.5 px-4 text-xs'>
                            {shortcuts.map((entry) => (
                                <div key={entry.id} className='flex items-center justify-between gap-2'>
                                    <span className='text-app-sidebar-icon-default truncate'>{t(entry.descriptionKey)}</span>
                                    <kbd className='shrink-0 opacity-70'>{formatKeymapShortcut(entry)}</kbd>
                                </div>
                            ))}
                        </CardContent>
                    </Card>
                )}
            </div>
        </div>
    )
}
