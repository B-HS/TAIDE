import type { FC, KeyboardEvent as ReactKeyboardEvent } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { Keyboard } from 'lucide-react'
import { toast } from 'sonner'
import { formatCategorizedLabel, listRegisteredCommands } from '@shared/lib/command-registry'
import {
    buildKeybindingRows,
    buildUnbindOverride,
    filterKeybindingRowsByCapturedKey,
    findConflictingRow,
    findKeybindingRowById,
    isKeybindingRowUnassigned,
    mergeKeybindingOverride,
    removeKeybindingOverride,
    sortKeybindingRows,
} from '@shared/lib/keybinding-catalog'
import type { KeymapChordStage, KeymapModifier } from '@shared/lib/keymap'
import {
    MODIFIER_ONLY_KEYS,
    captureModsFromEvent,
    formatKeymapShortcut,
    normalizeKeymapEventKey,
    parseKeymapOverrides,
    serializeKeymapOverrides,
} from '@shared/lib/keymap'
import { setKeymapCapturing } from '@shared/lib/keymap-capture'
import { clearKeymapChordState } from '@shared/lib/keymap-chord-store'
import { DEFAULT_KEYMAP_CONTEXT_GETTERS, getKeymapContextValue } from '@shared/lib/keymap-context'
import { isMonacoCommandId, resolveMonacoKeyCode } from '@shared/lib/monaco-keybinding'
import { fuzzyFilter } from '@shared/lib/fuzzy-match'
import { cn } from '@shared/lib/cn'
import { KeybindingRow } from '@features/settings/keybinding-row'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'

type CaptureTarget = { kind: 'row'; rowId: string; pendingFirstStage: KeymapChordStage | null } | { kind: 'search-key' }

const KEYMAP_CONTEXT_INSPECTOR_POLL_MS = 500

type KeybindingsEditorProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
}

export const KeybindingsEditor: FC<KeybindingsEditorProps> = ({ open, onOpenChange }) => {
    const [query, setQuery] = useState('')
    const [captureTarget, setCaptureTarget] = useState<CaptureTarget | null>(null)
    const [searchedKey, setSearchedKey] = useState<{ key: string; mods: KeymapModifier[] } | null>(null)
    const [showConflictsOnly, setShowConflictsOnly] = useState(false)
    const [showUnassignedOnly, setShowUnassignedOnly] = useState(false)
    const [activeContextKeys, setActiveContextKeys] = useState<string[]>([])

    const { t } = useTranslation()
    const { data: settings } = useQuery(settingsQueryOptions())
    const { mutate: updateSettings } = useUpdateSettings()

    const isKeySearchMode = captureTarget?.kind === 'search-key'
    const isCapturing = captureTarget !== null

    const resetSearchState = () => {
        setQuery('')
        setCaptureTarget(null)
        setSearchedKey(null)
        setShowConflictsOnly(false)
        setShowUnassignedOnly(false)
    }

    const handleOpenChange = (next: boolean) => {
        onOpenChange(next)
        if (!next) resetSearchState()
    }

    const overrides = parseKeymapOverrides(settings?.keymapOverrides ?? null)
    const rows = buildKeybindingRows(listRegisteredCommands(), overrides)
    const sortedRows = sortKeybindingRows(rows, (row) => formatCategorizedLabel(t, row.categoryKey, row.titleKey, row.titleDefaultValue ?? undefined))

    const textFilteredRows = query
        ? fuzzyFilter(
              query,
              sortedRows,
              (row) => `${formatCategorizedLabel(t, row.categoryKey, row.titleKey, row.titleDefaultValue ?? undefined)} ${row.id}`,
          ).map((ranked) => ranked.item)
        : sortedRows
    const keyFilteredRows = searchedKey ? filterKeybindingRowsByCapturedKey(textFilteredRows, searchedKey.key, searchedKey.mods) : textFilteredRows
    const conflictFilteredRows = showConflictsOnly ? keyFilteredRows.filter((row) => findConflictingRow(rows, row)) : keyFilteredRows
    const visibleRows = showUnassignedOnly ? conflictFilteredRows.filter(isKeybindingRowUnassigned) : conflictFilteredRows

    const conflictCount = rows.filter((row) => findConflictingRow(rows, row)).length
    const unassignedCount = rows.filter(isKeybindingRowUnassigned).length

    const saveOverrides = (nextOverrides: typeof overrides) =>
        updateSettings({ ...emptySettingsPatch(), keymapOverrides: serializeKeymapOverrides(nextOverrides) })

    const isKeyBindable = (rowId: string, key: string) => !isMonacoCommandId(rowId) || resolveMonacoKeyCode(key) !== null

    const handleChangeBinding = (rowId: string, key: string, mods: KeymapModifier[], chord?: KeymapChordStage) => {
        if (mods.length === 0) return
        if (chord && chord.mods.length === 0) return
        if (!isKeyBindable(rowId, key)) return toast.warning(t('settings.keymapKeyNotBindable'))
        if (chord && !isKeyBindable(rowId, chord.key)) return toast.warning(t('settings.keymapKeyNotBindable'))
        const currentRow = findKeybindingRowById(rows, rowId)
        const conflict = currentRow ? findConflictingRow(rows, { ...currentRow, key, mods, chord }) : null
        if (conflict)
            toast.warning(
                t('settings.keymapConflictWarning', {
                    action: formatCategorizedLabel(t, conflict.categoryKey, conflict.titleKey, conflict.titleDefaultValue ?? undefined),
                }),
            )
        saveOverrides(mergeKeybindingOverride(overrides, { actionId: rowId, key, mods, chord }))
        setCaptureTarget(null)
    }

    const handleCaptureStage = (rowId: string, key: string, mods: KeymapModifier[]) => {
        if (captureTarget?.kind !== 'row' || captureTarget.rowId !== rowId) return
        if (captureTarget.pendingFirstStage)
            return handleChangeBinding(rowId, captureTarget.pendingFirstStage.key, captureTarget.pendingFirstStage.mods, { key, mods })
        if (!isKeyBindable(rowId, key)) return toast.warning(t('settings.keymapKeyNotBindable'))
        setCaptureTarget({ ...captureTarget, pendingFirstStage: { key, mods } })
    }

    const handleConfirmSingleStage = (rowId: string) => {
        if (captureTarget?.kind !== 'row' || captureTarget.rowId !== rowId || !captureTarget.pendingFirstStage) return
        handleChangeBinding(rowId, captureTarget.pendingFirstStage.key, captureTarget.pendingFirstStage.mods)
    }

    const handleResetToDefault = (rowId: string) => saveOverrides(removeKeybindingOverride(overrides, rowId))
    const handleUnbind = (rowId: string) => saveOverrides(mergeKeybindingOverride(overrides, buildUnbindOverride(rowId)))

    const handleSearchKeyCaptureKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
        event.preventDefault()
        if (event.key === 'Escape') {
            setCaptureTarget(null)
            setSearchedKey(null)
            return
        }
        if (MODIFIER_ONLY_KEYS.includes(event.key)) return
        setSearchedKey({ key: normalizeKeymapEventKey(event), mods: captureModsFromEvent(event) })
    }

    const handleDialogEscapeKeyDown = (event: KeyboardEvent) => {
        if (!isCapturing) return
        event.preventDefault()
        setCaptureTarget(null)
        if (isKeySearchMode) setSearchedKey(null)
    }

    const toggleKeySearchMode = () => {
        if (isKeySearchMode) {
            setCaptureTarget(null)
            setSearchedKey(null)
            return
        }
        setSearchedKey(null)
        setCaptureTarget({ kind: 'search-key' })
    }

    useEffect(() => {
        setKeymapCapturing(isCapturing)
        if (isCapturing) clearKeymapChordState()
    }, [isCapturing])
    useEffect(() => () => setKeymapCapturing(false), [])
    /**
     * Radix's `Dialog` traps focus inside its content while `open` — `document.activeElement` never
     * sits inside `.monaco-editor`/`.xterm` for as long as the dialog is up, so polling *while open*
     * can only ever read "no active context key", making the inspector permanently empty (the exact
     * opposite of its purpose — debugging `when` context). Instead this keeps tracking in the
     * background *while the dialog is closed* and simply stops (via the effect cleanup below,
     * triggered by `open` flipping true) the instant it opens — so the panel shows a frozen snapshot
     * of whatever was focused right before the user opened it, which is the most recent real signal
     * available once focus is trapped. The `previous`/`next` array-equality check avoids publishing
     * a new array reference (and re-rendering this dialog's whole catalog+filter pipeline) every poll
     * when the context genuinely hasn't changed.
     */
    useEffect(() => {
        if (open) return
        const readActiveContextKeys = () =>
            setActiveContextKeys((previous) => {
                const next = Object.keys(DEFAULT_KEYMAP_CONTEXT_GETTERS).filter((key) => getKeymapContextValue(key))
                return previous.length === next.length && previous.every((key, index) => key === next[index]) ? previous : next
            })
        readActiveContextKeys()
        const intervalId = setInterval(readActiveContextKeys, KEYMAP_CONTEXT_INSPECTOR_POLL_MS)
        return () => clearInterval(intervalId)
    }, [open])

    return (
        <Dialog open={open} onOpenChange={handleOpenChange}>
            <DialogContent className='flex h-[70vh] flex-col overflow-hidden sm:max-w-3xl' onEscapeKeyDown={handleDialogEscapeKeyDown}>
                <DialogHeader>
                    <DialogTitle>{t('settings.keymapEditorTitle')}</DialogTitle>
                </DialogHeader>

                <div className='flex min-w-0 shrink-0 items-center gap-2'>
                    {isKeySearchMode ? (
                        <button
                            type='button'
                            autoFocus
                            onKeyDown={handleSearchKeyCaptureKeyDown}
                            onBlur={() => setCaptureTarget(null)}
                            className='border-app-focus-border bg-app-sidebar-item-active text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1 text-left font-mono'>
                            {searchedKey ? formatKeymapShortcut(searchedKey) : t('settings.keymapCapturePrompt')}
                        </button>
                    ) : (
                        <input
                            type='text'
                            value={query}
                            onChange={(event) => setQuery(event.target.value)}
                            placeholder={t('settings.keymapSearchPlaceholder')}
                            aria-label={t('settings.keymapSearchAriaLabel')}
                            className='bg-panel-input-background border-panel-input-border text-app-foreground min-w-0 flex-1 rounded-sm border px-2 py-1'
                        />
                    )}
                    <Button type='button' variant={isKeySearchMode ? 'default' : 'outline'} size='xs' onClick={toggleKeySearchMode}>
                        <Keyboard className='size-3.5' />
                        {t('settings.keymapSearchByKey')}
                    </Button>
                </div>

                <div className='flex shrink-0 items-center gap-2 text-xs'>
                    <button
                        type='button'
                        onClick={() => setShowConflictsOnly((value) => !value)}
                        className={cn(
                            'border-app-border rounded-full border px-2 py-0.5',
                            showConflictsOnly ? 'bg-status-warning/15 text-status-warning border-status-warning/40' : 'text-app-sidebar-icon-default',
                        )}>
                        {t('settings.keymapConflictFilter')} ({conflictCount})
                    </button>
                    <button
                        type='button'
                        onClick={() => setShowUnassignedOnly((value) => !value)}
                        className={cn(
                            'border-app-border rounded-full border px-2 py-0.5',
                            showUnassignedOnly ? 'bg-app-sidebar-item-active text-app-foreground' : 'text-app-sidebar-icon-default',
                        )}>
                        {t('settings.keymapUnassignedFilter')} ({unassignedCount})
                    </button>
                </div>

                <div className='text-app-sidebar-icon-default flex shrink-0 flex-wrap items-center gap-2 text-[10px]'>
                    <span className='tracking-wide uppercase'>{t('settings.keymapInspectorTitle')}</span>
                    {activeContextKeys.length === 0 ? (
                        <span>{t('settings.keymapInspectorEmpty')}</span>
                    ) : (
                        activeContextKeys.map((key) => (
                            <span key={key} className='bg-app-sidebar-item-active text-app-foreground rounded-full px-1.5 py-0.5 font-mono'>
                                {key}
                            </span>
                        ))
                    )}
                </div>

                <div className='text-app-sidebar-icon-default flex shrink-0 items-center gap-3 px-3 text-[10px] tracking-wide uppercase'>
                    <span className='flex-1'>{t('settings.keymapCommandColumn')}</span>
                    <span>{t('settings.keymapKeyColumn')}</span>
                    <span>{t('settings.keymapSourceColumn')}</span>
                </div>

                <ScrollContainer className='min-h-0 flex-1'>
                    {visibleRows.length === 0 ? (
                        <p className='text-app-sidebar-icon-default px-1 py-4 text-center text-xs'>{t('settings.keymapNoResults')}</p>
                    ) : (
                        <ul className='flex flex-col gap-1 pr-2'>
                            {visibleRows.map((row) => {
                                const conflict = findConflictingRow(rows, row)
                                return (
                                    <KeybindingRow
                                        key={row.id}
                                        row={row}
                                        isCapturing={captureTarget?.kind === 'row' && captureTarget.rowId === row.id}
                                        pendingChordFirstStage={
                                            captureTarget?.kind === 'row' && captureTarget.rowId === row.id ? captureTarget.pendingFirstStage : null
                                        }
                                        conflictLabel={
                                            conflict
                                                ? formatCategorizedLabel(
                                                      t,
                                                      conflict.categoryKey,
                                                      conflict.titleKey,
                                                      conflict.titleDefaultValue ?? undefined,
                                                  )
                                                : null
                                        }
                                        onStartCapture={() => setCaptureTarget({ kind: 'row', rowId: row.id, pendingFirstStage: null })}
                                        onCaptureStage={(key, mods) => handleCaptureStage(row.id, key, mods)}
                                        onConfirmSingleStage={() => handleConfirmSingleStage(row.id)}
                                        onCancelCapture={() => setCaptureTarget(null)}
                                        onResetToDefault={() => handleResetToDefault(row.id)}
                                        onUnbind={() => handleUnbind(row.id)}
                                        onResolveConflict={() => conflict && handleUnbind(conflict.id)}
                                    />
                                )
                            })}
                        </ul>
                    )}
                </ScrollContainer>
            </DialogContent>
        </Dialog>
    )
}
