import type { FC } from 'react'
import { useEffect, useRef, useState, useSyncExternalStore } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { LspSessionStatus, PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { getEditorInstance, subscribeEditorInstance } from '@entities/editor/editor-instance-registry'
import { layoutQueryOptions } from '@entities/layout/layout.query'
import { lspSessionsQueryOptions } from '@entities/lsp/lsp.query'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { emptySettingsPatch, getSettings } from '@entities/settings/settings.ipc'
import { settingsQueryOptions, useUpdateSettings } from '@entities/settings/settings.query'
import { systemUsageQueryOptions } from '@entities/system/system.query'
import { toProblemSeverity } from '@features/problems/problem-severity'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { findActiveTab } from '@shared/lib/pane-tree'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { CODE_FONT_SIZE_STEP, DEFAULT_CODE_FONT_SIZE, MAX_CODE_FONT_SIZE, MIN_CODE_FONT_SIZE } from '@shared/constants/code-font-size'
import { QUERY_KEY } from '@shared/constants/query-key'
import { APP_KEYMAP, applyKeymapOverrides, parseKeymapOverrides } from '@shared/lib/keymap'
import { getModel } from '@entities/editor/model-registry'
import { saveFile } from '@entities/file/file.ipc'
import { openTab, setTabDirty } from '@entities/layout/layout.ipc'
import { ideStatusQueryOptions, useIdeStatusSync } from '@entities/ide/ide.query'
import { publishIdeDiagnostics, resolveIdeDiff, resolveIdeSave } from '@entities/ide/ide.ipc'
import { removePendingClaudeDiff, setPendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { StatusBar } from '@features/window/status-bar'
import { SystemUsageModal } from '@widgets/system-usage-modal/system-usage-modal'

const IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS = 300

const clampFontSize = (value: number) => Math.min(MAX_CODE_FONT_SIZE, Math.max(MIN_CODE_FONT_SIZE, value))

const isLspRunning = (status: LspSessionStatus) => status === 'running'

const isLspCrashed = (status: LspSessionStatus) => status === 'crashed'

const findFileTabByPath = (node: PaneNode, path: string): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.kind.kind === 'file' && tab.kind.path === path) ?? null
    for (const child of node.children) {
        const found = findFileTabByPath(child, path)
        if (found) return found
    }
    return null
}

type StatusBarContentProps = {
    isProblemsOpen: boolean
    onToggleProblems: () => void
}

export const StatusBarContent: FC<StatusBarContentProps> = ({ isProblemsOpen, onToggleProblems }) => {
    const cursorSnapshotRef = useRef<{ line: number; column: number } | null>(null)

    const [isUsageModalOpen, setUsageModalOpen] = useState(false)

    const queryClient = useQueryClient()
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

    const keymapEntries = applyKeymapOverrides(APP_KEYMAP, parseKeymapOverrides(settings?.keymapOverrides ?? null))

    useGlobalKeymap({ 'font-size-up': increaseEditorFontSize, 'font-size-down': decreaseEditorFontSize }, keymapEntries)

    useTauriEvent(events.ideDiffRequested, ({ payload }) => {
        setPendingClaudeDiff(payload.requestId, { oldPath: payload.oldPath, newContents: payload.newContents, tabName: payload.tabName })

        void (async () => {
            const current = settings ?? (await getSettings())
            if (!current.ideAutoOpenDiff) {
                removePendingClaudeDiff(payload.requestId)
                await resolveIdeDiff({ requestId: payload.requestId, outcome: 'rejected', content: null })
                return
            }

            const layout = await openTab({
                projectId: payload.projectId,
                kind: { kind: 'claudeDiff', requestId: payload.requestId, path: payload.newPath },
                title: payload.tabName,
                target: null,
                preview: false,
            })
            queryClient.setQueryData(QUERY_KEY.LAYOUT.DETAIL(payload.projectId), layout)
        })().catch((error: unknown) => toast.error(error instanceof Error ? error.message : String(error)))
    })

    useTauriEvent(events.ideSaveRequested, ({ payload }) => {
        void (async () => {
            const layout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(payload.projectId))
            const tab = layout ? findFileTabByPath(layout.root, payload.path) : null

            if (!tab?.dirty) {
                await resolveIdeSave({ requestId: payload.requestId, saved: true }).catch(() => undefined)
                return
            }

            const model = getModel(payload.path)
            if (!model) {
                await resolveIdeSave({ requestId: payload.requestId, saved: false }).catch(() => undefined)
                return
            }

            try {
                await saveFile({ path: payload.path, content: model.getValue() })
                await setTabDirty({ tabId: tab.id, dirty: false })
                void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.CONTENT(payload.path) })
                await resolveIdeSave({ requestId: payload.requestId, saved: true })
            } catch {
                await resolveIdeSave({ requestId: payload.requestId, saved: false }).catch(() => undefined)
            }
        })()
    })

    useTauriEvent(events.ideCloseTabRequested, ({ payload }) => {
        if (payload.requestId) removePendingClaudeDiff(payload.requestId)
    })

    useIdeStatusSync()

    useEffect(() => {
        if (!activeProjectId || !ideStatus?.running) return

        const timeout = setTimeout(() => {
            const items = markers.map((marker) => {
                const range = monacoRangeToLsp(marker)
                return {
                    path: marker.resource.fsPath,
                    severity: toProblemSeverity(marker.severity),
                    startLine: range.start.line,
                    startCharacter: range.start.character,
                    endLine: range.end.line,
                    endCharacter: range.end.character,
                    message: marker.message,
                    source: marker.source ?? null,
                }
            })
            void publishIdeDiagnostics({ projectId: activeProjectId, items })
        }, IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS)

        return () => clearTimeout(timeout)
    }, [markers, activeProjectId, ideStatus?.running])

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
