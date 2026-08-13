import type { FC } from 'react'
import { Activity, CheckCircle2, CircleX, Plug, PlugZap, SquareTerminal, Type, Unplug, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import type { IdeStatus, SystemUsage } from '@shared/api/bindings'
import { BYTES_PER_MEBIBYTE } from '@shared/constants/system-usage'
import { cn } from '@shared/lib/cn'
import { IconButton } from '@shared/ui/icon-button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { FontSizeStepper } from '@features/window/font-size-stepper'

type LspSummary = {
    running: number
    total: number
    hasCrashed: boolean
}

type IdeConnectionState = 'connected' | 'waiting' | 'notRunning'

const resolveIdeConnectionState = (status: IdeStatus | null): IdeConnectionState => {
    if (status?.connected) return 'connected'
    if (status?.running) return 'waiting'
    return 'notRunning'
}

const IDE_STATE_ICON = {
    connected: PlugZap,
    waiting: Plug,
    notRunning: Unplug,
} as const

const IDE_STATE_LABEL_KEY = {
    connected: 'ide.connected',
    waiting: 'ide.starting',
    notRunning: 'ide.disconnected',
} as const

type CursorPosition = {
    line: number
    column: number
}

type StatusBarProps = {
    lspSummary: LspSummary | null
    errorCount: number
    isProblemsOpen: boolean
    onToggleProblems: () => void
    systemUsage: SystemUsage | null
    ideStatus: IdeStatus | null
    cursorPosition: CursorPosition | null
    editorFontSize: number
    terminalFontSize: number
    onEditorFontSizeDecrease: () => void
    onEditorFontSizeIncrease: () => void
    onEditorFontSizeReset: () => void
    onTerminalFontSizeDecrease: () => void
    onTerminalFontSizeIncrease: () => void
    onTerminalFontSizeReset: () => void
}

export const StatusBar: FC<StatusBarProps> = ({
    lspSummary,
    errorCount,
    isProblemsOpen,
    onToggleProblems,
    systemUsage,
    ideStatus,
    cursorPosition,
    editorFontSize,
    terminalFontSize,
    onEditorFontSizeDecrease,
    onEditorFontSizeIncrease,
    onEditorFontSizeReset,
    onTerminalFontSizeDecrease,
    onTerminalFontSizeIncrease,
    onTerminalFontSizeReset,
}) => {
    const { t } = useTranslation()

    const ideConnectionState = resolveIdeConnectionState(ideStatus)
    const IdeStateIcon = IDE_STATE_ICON[ideConnectionState]
    const ideTitle = ideConnectionState === 'notRunning' ? undefined : t('ide.title', { port: ideStatus?.port ?? 0 })

    return (
        <div className='bg-app-sidebar-background border-app-border text-app-sidebar-icon-default flex h-6 shrink-0 items-center justify-between gap-3 border-t px-2 text-[11px] select-none'>
            <div className='flex min-w-0 items-center gap-3'>
                <IconButton
                    label={t('problems.toggleAriaLabel')}
                    icon={
                        <>
                            <CircleX className='size-3' />
                            <span className='tabular-nums'>{errorCount}</span>
                        </>
                    }
                    aria-pressed={isProblemsOpen}
                    onClick={onToggleProblems}
                    side='top'
                    className={cn(
                        'flex shrink-0 items-center gap-1 rounded-sm px-1',
                        errorCount > 0 ? 'text-status-error' : 'text-app-sidebar-icon-default',
                        isProblemsOpen ? 'bg-explorer-item-selected' : 'hover:bg-explorer-item-hover',
                    )}
                />
                {lspSummary && (
                    <span className={cn('flex shrink-0 items-center gap-1', lspSummary.hasCrashed ? 'text-status-error' : 'text-status-success')}>
                        {lspSummary.hasCrashed ? <XCircle className='size-3' /> : <CheckCircle2 className='size-3' />}
                        {t('window.lspStatus', { running: lspSummary.running, total: lspSummary.total })}
                    </span>
                )}
                {ideTitle ? (
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <span
                                className={cn(
                                    'flex shrink-0 items-center gap-1',
                                    ideConnectionState === 'connected' ? 'text-status-success' : 'text-app-sidebar-icon-default',
                                )}>
                                <IdeStateIcon className='size-3' />
                                {t(IDE_STATE_LABEL_KEY[ideConnectionState])}
                            </span>
                        </TooltipTrigger>
                        <TooltipContent side='top'>{ideTitle}</TooltipContent>
                    </Tooltip>
                ) : (
                    <span
                        className={cn(
                            'flex shrink-0 items-center gap-1',
                            ideConnectionState === 'connected' ? 'text-status-success' : 'text-app-sidebar-icon-default',
                        )}>
                        <IdeStateIcon className='size-3' />
                        {t(IDE_STATE_LABEL_KEY[ideConnectionState])}
                    </span>
                )}
            </div>
            <div className='flex shrink-0 items-center gap-3'>
                {cursorPosition && (
                    <span className='text-app-sidebar-icon-default shrink-0 tabular-nums'>
                        {t('editor.cursorPosition', { line: cursorPosition.line, column: cursorPosition.column })}
                    </span>
                )}
                {systemUsage && (
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <span className='text-app-sidebar-icon-default flex shrink-0 items-center gap-1'>
                                <Activity className='size-3' />
                                {t('window.systemUsage', {
                                    cpu: systemUsage.cpuPercent === null ? '--' : Math.round(systemUsage.cpuPercent),
                                    memory: Math.round((systemUsage.memoryBytes ?? 0) / BYTES_PER_MEBIBYTE),
                                })}
                            </span>
                        </TooltipTrigger>
                        <TooltipContent side='top'>{t('window.systemUsageHint')}</TooltipContent>
                    </Tooltip>
                )}
                <FontSizeStepper
                    label={t('window.editorFontSize')}
                    icon={<Type className='size-3' />}
                    value={editorFontSize}
                    onDecrease={onEditorFontSizeDecrease}
                    onIncrease={onEditorFontSizeIncrease}
                    onReset={onEditorFontSizeReset}
                />
                <FontSizeStepper
                    label={t('window.terminalFontSize')}
                    icon={<SquareTerminal className='size-3' />}
                    value={terminalFontSize}
                    onDecrease={onTerminalFontSizeDecrease}
                    onIncrease={onTerminalFontSizeIncrease}
                    onReset={onTerminalFontSizeReset}
                />
            </div>
        </div>
    )
}
