import type { FC, KeyboardEvent } from 'react'
import { RotateCcw, TriangleAlert, Unlink } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { KeybindingRow as KeybindingRowData } from '@shared/lib/keymap/keybinding-catalog'
import { isKeybindingRowUnassigned } from '@shared/lib/keymap/keybinding-catalog'
import { formatCategorizedLabel } from '@shared/lib/command-registry'
import type { KeymapChordStage, KeymapModifier } from '@shared/lib/keymap/keymap'
import { MODIFIER_ONLY_KEYS, captureModsFromEvent, formatKeymapShortcut, normalizeKeymapEventKey } from '@shared/lib/keymap/keymap'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

type KeybindingRowProps = {
    row: KeybindingRowData
    isCapturing: boolean
    pendingChordFirstStage: KeymapChordStage | null
    conflictLabel: string | null
    onStartCapture: () => void
    onCaptureStage: (key: string, mods: KeymapModifier[]) => void
    onConfirmSingleStage: () => void
    onCancelCapture: () => void
    onResetToDefault: () => void
    onUnbind: () => void
    onResolveConflict: () => void
}

export const KeybindingRow: FC<KeybindingRowProps> = ({
    row,
    isCapturing,
    pendingChordFirstStage,
    conflictLabel,
    onStartCapture,
    onCaptureStage,
    onConfirmSingleStage,
    onCancelCapture,
    onResetToDefault,
    onUnbind,
    onResolveConflict,
}) => {
    const { t } = useTranslation()
    const isUnassigned = isKeybindingRowUnassigned(row)
    const assignedBindingLabel = row.key ? formatKeymapShortcut(row) : (row.defaultBindingLabel ?? '')

    const handleCaptureKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
        event.preventDefault()
        if (event.key === 'Escape') return onCancelCapture()
        if (MODIFIER_ONLY_KEYS.includes(event.key)) return
        const mods = captureModsFromEvent(event)
        /**
         * A bare Enter (no modifiers held) confirms the pending first stage as a single-key bind —
         * but a *modified* Enter (Cmd+Enter, Shift+Enter, ...) is a capturable stage key in its own
         * right (monaco maps `enter` to a real `KeyCode`), so only the unmodified case short-circuits
         * here. Without this, "confirm" would win unconditionally and a chord's second stage could
         * never be Enter at all.
         */
        if (pendingChordFirstStage && event.key === 'Enter' && mods.length === 0) return onConfirmSingleStage()
        /**
         * Only a *first* stage must carry a modifier (an unmodified single key would swallow plain
         * typing). A chord's second stage is already scoped by its prefix, so a bare key is valid
         * there — `APP_KEYMAP` ships one itself (⌘K Z, `toggle-zen-mode`), which this capture UI
         * could not reproduce while the modifier requirement applied to both stages.
         */
        if (!pendingChordFirstStage && mods.length === 0) return toast.warning(t('settings.keymapModifierRequired'))
        onCaptureStage(normalizeKeymapEventKey(event), mods)
    }

    return (
        <li className='border-app-border grid min-w-0 grid-cols-[1fr_auto_auto_auto] items-center gap-3 rounded-sm border px-3 py-1.5 text-xs'>
            <div className='flex min-w-0 flex-col gap-0.5'>
                <span className='text-app-foreground truncate'>
                    {formatCategorizedLabel(t, row.categoryKey, row.titleKey, row.titleDefaultValue ?? undefined)}
                </span>
                {conflictLabel && (
                    <span className='text-status-warning flex min-w-0 items-center gap-1'>
                        <TriangleAlert className='size-3 shrink-0' />
                        <span className='truncate'>{t('settings.keymapConflictWarning', { action: conflictLabel })}</span>
                        <Button type='button' variant='ghost' size='xs' onClick={onResolveConflict}>
                            {t('settings.keymapUnbind')}
                        </Button>
                    </span>
                )}
            </div>

            {isCapturing ? (
                <span className='flex shrink-0 items-center gap-1.5'>
                    <button
                        type='button'
                        autoFocus
                        onKeyDown={handleCaptureKeyDown}
                        onBlur={onCancelCapture}
                        className='border-app-focus-border bg-app-sidebar-item-active text-app-foreground shrink-0 rounded-sm border px-2 py-1 font-mono'>
                        {pendingChordFirstStage
                            ? t('settings.keymapChordCapturePrompt', { shortcut: formatKeymapShortcut(pendingChordFirstStage) })
                            : t('settings.keymapCapturePrompt')}
                    </button>
                    {pendingChordFirstStage && (
                        <>
                            <span className='bg-app-accent/15 text-app-accent shrink-0 rounded-sm px-1 py-0.5 text-[10px]'>
                                {t('settings.keymapChordWaitingBadge')}
                            </span>
                            <Button
                                type='button'
                                variant='ghost'
                                size='xs'
                                onMouseDown={(event) => event.preventDefault()}
                                onClick={onConfirmSingleStage}>
                                {t('settings.keymapChordConfirmSingle')}
                            </Button>
                        </>
                    )}
                </span>
            ) : (
                <span className='flex shrink-0 items-center gap-1.5'>
                    <span
                        className={cn(
                            'text-app-foreground font-mono',
                            row.isOverridden && 'text-app-accent',
                            isUnassigned && 'text-app-sidebar-icon-default',
                        )}>
                        {isUnassigned ? t('settings.keymapUnassigned') : assignedBindingLabel}
                    </span>
                    {conflictLabel && (
                        <span className='bg-status-warning/15 text-status-warning rounded-sm px-1 py-0.5 text-[10px]'>
                            {t('settings.keymapConflictBadge')}
                        </span>
                    )}
                </span>
            )}

            <span className='text-app-sidebar-icon-default shrink-0'>
                {row.isOverridden ? t('settings.keymapSourceUser') : t('settings.keymapSourceDefault')}
            </span>

            <div className='flex shrink-0 items-center gap-1'>
                <Button type='button' variant='outline' size='xs' onClick={onStartCapture}>
                    {t('settings.keymapChange')}
                </Button>
                {row.isOverridden && (
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button type='button' variant='ghost' size='icon-xs' aria-label={t('settings.keymapResetOne')} onClick={onResetToDefault}>
                                <RotateCcw className='size-3' />
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side='bottom'>{t('settings.keymapResetOne')}</TooltipContent>
                    </Tooltip>
                )}
                {(row.key || row.defaultBindingLabel) && (
                    <Tooltip>
                        <TooltipTrigger asChild>
                            <Button type='button' variant='ghost' size='icon-xs' aria-label={t('settings.keymapUnbind')} onClick={onUnbind}>
                                <Unlink className='size-3' />
                            </Button>
                        </TooltipTrigger>
                        <TooltipContent side='bottom'>{t('settings.keymapUnbind')}</TooltipContent>
                    </Tooltip>
                )}
            </div>
        </li>
    )
}
