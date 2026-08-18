import type { FC } from 'react'
import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useQuery } from '@tanstack/react-query'
import type { LspSessionStatus } from '@shared/api/bindings'
import { getEditorInstance, subscribeEditorInstance } from '@entities/editor/editor-instance-registry'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { lspSessionsQueryOptions } from '@entities/lsp/lsp.query'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { emptySettingsPatch } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { systemUsageQueryOptions } from '@entities/system/system.query'
import { toProblemSeverity } from '@features/problems/problem-severity'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { findActiveTab } from '@shared/lib/pane-tree'
import { MONACO_CHORD_PREFIX_KEY, formatKeymapShortcut } from '@shared/lib/keymap'
import { getKeymapChordStoreSnapshot, subscribeKeymapChordNoMatch, subscribeKeymapChordStore } from '@shared/lib/keymap-chord-store'
import { CODE_FONT_SIZE_STEP, DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { ideStatusQueryOptions } from '@entities/ide/ide.query'
import { StatusBar } from '@features/window/status-bar'
import { SystemUsageModal } from '@widgets/system-usage-modal/system-usage-modal'

const CHORD_NO_MATCH_INDICATOR_DURATION_MS = 1500

const clampFontSize = (value: number) => Math.min(MAX_CODE_FONT_SIZE, Math.max(MIN_CODE_FONT_SIZE, value))

const isLspRunning = (status: LspSessionStatus) => status === 'running'

const isLspCrashed = (status: LspSessionStatus) => status === 'crashed'

type StatusBarContentProps = {
    isProblemsOpen: boolean
    onToggleProblems: () => void
}

export const StatusBarContent: FC<StatusBarContentProps> = ({ isProblemsOpen, onToggleProblems }) => {
    const cursorSnapshotRef = useRef<{ line: number; column: number } | null>(null)
    const chordNoMatchTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

    const [isUsageModalOpen, setUsageModalOpen] = useState(false)
    const [chordNoMatchFlash, setChordNoMatchFlash] = useState(false)

    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: ideStatus = null } = useQuery(ideStatusQueryOptions())
    const { data: layout } = useQuery(layoutQueryOptions(activeProjectId))
    const { data: lspSessions = [] } = useQuery(lspSessionsQueryOptions(activeProjectId))
    const showSystemUsage = settings?.showSystemUsage ?? true
    const { data: systemUsage = null } = useQuery(systemUsageQueryOptions(showSystemUsage))
    const { mutate: updateSettings } = useUpdateSettings()
    const markers = useMonacoMarkers()

    const editorFontSize = settings?.editorFontSize ?? DEFAULT_CODE_FONT_SIZE
    const terminalFontSize = settings?.terminalFontSize ?? DEFAULT_CODE_FONT_SIZE
    const errorCount = markers.filter((marker) => toProblemSeverity(marker.severity) === 'error').length
    const focusedTabId = layout ? findActiveTab(layout.root, layout.focusedPane)?.id : undefined

    const getCursorSnapshot = () => {
        const editor = focusedTabId ? getEditorInstance(focusedTabId) : null
        const position = editor?.getPosition()
        if (!position) {
            cursorSnapshotRef.current = null
            return null
        }
        const cached = cursorSnapshotRef.current
        if (cached && cached.line === position.lineNumber && cached.column === position.column) return cached
        const next = { line: position.lineNumber, column: position.column }
        cursorSnapshotRef.current = next
        return next
    }

    const subscribeToCursor = (onStoreChange: () => void) => {
        if (!focusedTabId) return () => {}

        let cursorSubscription: { dispose: () => void } | null = null

        const attachToEditor = () => {
            cursorSubscription?.dispose()
            const editor = getEditorInstance(focusedTabId)
            cursorSubscription = editor?.onDidChangeCursorPosition(onStoreChange) ?? null
            onStoreChange()
        }

        attachToEditor()
        const editorSubscription = subscribeEditorInstance(focusedTabId, attachToEditor)

        return () => {
            editorSubscription()
            cursorSubscription?.dispose()
        }
    }

    const cursorPosition = useSyncExternalStore(subscribeToCursor, getCursorSnapshot)
    const chordState = useSyncExternalStore(subscribeKeymapChordStore, getKeymapChordStoreSnapshot)
    /**
     * The monaco-deferral window has no indicator of its own on monaco's side (Wave H contract §2.4
     * — standalone's own chord-status display is a no-op stub), so this is the *only* feedback the
     * user gets while it's armed. Reuses the same "waiting for next key" presentation as an app chord
     * wait — `MONACO_CHORD_PREFIX_KEY` for the shortcut label — since both mean the same thing to the
     * user: a keydown is coming, and the next key needs to land on a specific target.
     */
    const resolveChordPendingShortcut = () => {
        if (chordState.pending) return formatKeymapShortcut(chordState.pending.prefix)
        if (chordState.monacoDeferral) return formatKeymapShortcut(MONACO_CHORD_PREFIX_KEY)
        return null
    }
    const chordPendingShortcut = resolveChordPendingShortcut()

    const lspSummary =
        lspSessions.length > 0
            ? {
                  running: lspSessions.filter((session) => isLspRunning(session.status)).length,
                  total: lspSessions.length,
                  hasCrashed: lspSessions.some((session) => isLspCrashed(session.status)),
              }
            : null

    const increaseEditorFontSize = () =>
        updateSettings({ ...emptySettingsPatch(), editorFontSize: clampFontSize(editorFontSize + CODE_FONT_SIZE_STEP) })

    const decreaseEditorFontSize = () =>
        updateSettings({ ...emptySettingsPatch(), editorFontSize: clampFontSize(editorFontSize - CODE_FONT_SIZE_STEP) })

    useGlobalKeymap({ 'font-size-up': increaseEditorFontSize, 'font-size-down': decreaseEditorFontSize })

    useEffect(() => {
        const unsubscribe = subscribeKeymapChordNoMatch(() => {
            if (chordNoMatchTimeoutRef.current) clearTimeout(chordNoMatchTimeoutRef.current)
            setChordNoMatchFlash(true)
            chordNoMatchTimeoutRef.current = setTimeout(() => setChordNoMatchFlash(false), CHORD_NO_MATCH_INDICATOR_DURATION_MS)
        })
        return () => {
            unsubscribe()
            if (chordNoMatchTimeoutRef.current) clearTimeout(chordNoMatchTimeoutRef.current)
        }
    }, [])

    return (
        <>
            <StatusBar
                lspSummary={lspSummary}
                errorCount={errorCount}
                isProblemsOpen={isProblemsOpen}
                onToggleProblems={onToggleProblems}
                systemUsage={showSystemUsage ? systemUsage : null}
                onOpenUsageDetail={() => setUsageModalOpen(true)}
                ideStatus={ideStatus}
                cursorPosition={cursorPosition}
                chordPendingShortcut={chordPendingShortcut}
                chordNoMatchFlash={chordNoMatchFlash}
                editorFontSize={editorFontSize}
                terminalFontSize={terminalFontSize}
                onEditorFontSizeDecrease={decreaseEditorFontSize}
                onEditorFontSizeIncrease={increaseEditorFontSize}
                onEditorFontSizeReset={() => updateSettings({ ...emptySettingsPatch(), editorFontSize: DEFAULT_CODE_FONT_SIZE })}
                onTerminalFontSizeDecrease={() =>
                    updateSettings({ ...emptySettingsPatch(), terminalFontSize: clampFontSize(terminalFontSize - CODE_FONT_SIZE_STEP) })
                }
                onTerminalFontSizeIncrease={() =>
                    updateSettings({ ...emptySettingsPatch(), terminalFontSize: clampFontSize(terminalFontSize + CODE_FONT_SIZE_STEP) })
                }
                onTerminalFontSizeReset={() => updateSettings({ ...emptySettingsPatch(), terminalFontSize: DEFAULT_CODE_FONT_SIZE })}
            />
            <SystemUsageModal open={isUsageModalOpen} onOpenChange={setUsageModalOpen} />
        </>
    )
}
