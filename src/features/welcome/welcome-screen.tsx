import type { FC } from 'react'
import { Clock, FileText, FolderOpen, Keyboard, TriangleAlert } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { Project } from '@shared/api/bindings'
import { IS_MAC, MOD_KEY_LABEL } from '@shared/constants/platform'
import type { KeymapActionId, KeymapEntry } from '@shared/lib/keymap'
import { APP_KEYMAP, formatKeymapShortcut } from '@shared/lib/keymap'
import { Button } from '@shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@shared/ui/card'

/** A representative slice of `APP_KEYMAP` surfaced on the Welcome screen's shortcuts card — kept
 *  short (navigation/panel/save) rather than exhaustive; the full catalog lives in the keybindings
 *  editor (`settings.keymapOpenEditor`). Ids only, so the label/shortcut text is always derived
 *  from `APP_KEYMAP` itself and never hand-duplicated. */
const WELCOME_KEYMAP_HIGHLIGHT_IDS: KeymapActionId[] = ['quick-open', 'command-palette', 'search', 'toggle-terminal', 'toggle-sidebar', 'save']

type WelcomeScreenProps = {
    recentProjects: Project[]
    onOpenFolder: () => void
    /** Whether a file-open dialog has a project root to resolve against — false on the very first
     *  launch (no project open anywhere yet), true once this Welcome surface is scoped to one
     *  (either the zero-projects screen after a folder was opened, or a `welcome` tab inside an
     *  already-open project). */
    canOpenFile: boolean
    onOpenFile: () => void
    onSelectRecent: (project: Project) => void
}

export const WelcomeScreen: FC<WelcomeScreenProps> = ({ recentProjects, onOpenFolder, canOpenFile, onOpenFile, onSelectRecent }) => {
    const highlightedShortcuts = WELCOME_KEYMAP_HIGHLIGHT_IDS.map((id) => APP_KEYMAP.find((entry) => entry.id === id)).filter(
        (entry): entry is KeymapEntry => entry != null,
    )

    const { t } = useTranslation()

    return (
        <div className='bg-app-background flex h-full w-full items-center justify-center overflow-y-auto'>
            <div className='flex w-96 flex-col gap-6 py-10'>
                <div className='flex flex-col gap-1'>
                    <span className='text-2xl font-semibold tracking-tight'>TAIDE</span>
                    <span className='text-app-sidebar-icon-default'>{t('app.openFolderHint')}</span>
                </div>

                <div className='flex flex-col gap-2'>
                    <div className='flex flex-wrap items-center gap-2'>
                        <Button onClick={onOpenFolder} className='w-fit'>
                            <FolderOpen />
                            {t('app.openFolder')}
                            <kbd className='ml-2 text-xs opacity-70'>{MOD_KEY_LABEL}O</kbd>
                        </Button>
                        <Button onClick={onOpenFile} disabled={!canOpenFile} variant='outline' className='w-fit'>
                            <FileText />
                            {t('app.openFile')}
                        </Button>
                    </div>
                    {!canOpenFile && <span className='text-app-sidebar-icon-default text-xs'>{t('app.openFileHint')}</span>}
                </div>

                {recentProjects.length > 0 && (
                    <div className='flex flex-col gap-2'>
                        <span className='text-panel-section-header flex items-center gap-1.5 text-xs font-medium'>
                            <Clock className='size-3.5' />
                            {t('app.recentItems')}
                        </span>
                        <div className='flex flex-col'>
                            {recentProjects.map((project) => (
                                <button
                                    key={project.id}
                                    type='button'
                                    disabled={project.rootMissing}
                                    onClick={() => onSelectRecent(project)}
                                    className='hover:bg-app-sidebar-item-hover flex flex-col items-start rounded-md px-2 py-1 text-left disabled:cursor-not-allowed disabled:opacity-50'>
                                    <span>{project.name}</span>
                                    <span className='text-app-sidebar-icon-default flex w-full items-center gap-1 truncate text-xs'>
                                        {project.rootMissing && <TriangleAlert className='size-3 shrink-0' />}
                                        {project.rootMissing ? t('app.recentProjectRootMissing') : project.root}
                                    </span>
                                </button>
                            ))}
                        </div>
                    </div>
                )}

                {highlightedShortcuts.length > 0 && (
                    <Card className='py-4'>
                        <CardHeader className='px-4'>
                            <CardTitle className='flex items-center gap-1.5 text-xs'>
                                <Keyboard className='size-3.5' />
                                {t('app.keyboardShortcutsTitle')}
                            </CardTitle>
                        </CardHeader>
                        <CardContent className='grid grid-cols-2 gap-x-4 gap-y-1.5 px-4 text-xs'>
                            {highlightedShortcuts.map((entry) => (
                                <div key={entry.id} className='flex items-center justify-between gap-2'>
                                    <span className='text-app-sidebar-icon-default truncate'>{t(entry.descriptionKey)}</span>
                                    <kbd className='shrink-0 opacity-70'>{formatKeymapShortcut(entry, IS_MAC)}</kbd>
                                </div>
                            ))}
                        </CardContent>
                    </Card>
                )}
            </div>
        </div>
    )
}
