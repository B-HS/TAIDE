import type { FC } from 'react'
import { CheckCircle2, SquareTerminal, Type, XCircle } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { FontSizeStepper } from '@features/window/font-size-stepper'

type LspSummary = {
    running: number
    total: number
    hasCrashed: boolean
}

type StatusBarProps = {
    lspSummary: LspSummary | null
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

    return (
        <div className='bg-app-sidebar-background border-app-border text-app-sidebar-icon-default flex h-6 shrink-0 items-center justify-between gap-3 border-t px-2 text-[11px] select-none'>
            <div className='flex min-w-0 items-center gap-3'>
                {lspSummary && (
                    <span className={cn('flex shrink-0 items-center gap-1', lspSummary.hasCrashed ? 'text-status-error' : 'text-status-success')}>
                        {lspSummary.hasCrashed ? <XCircle className='size-3' /> : <CheckCircle2 className='size-3' />}
                        {t('window.lspStatus', { running: lspSummary.running, total: lspSummary.total })}
                    </span>
                )}
            </div>
            <div className='flex shrink-0 items-center gap-3'>
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
