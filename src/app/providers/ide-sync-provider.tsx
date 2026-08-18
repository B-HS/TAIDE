import type { FC, PropsWithChildren } from 'react'
import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { getModel } from '@entities/editor/model-registry'
import { saveFile } from '@entities/file/file.ipc'
import { removePendingClaudeDiff, setPendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { publishIdeDiagnostics, resolveIdeDiff, resolveIdeSave } from '@entities/ide/ide.ipc'
import { ideStatusQueryOptions, useIdeStatusSync } from '@entities/ide/ide.query'
import { openTab, setTabDirty } from '@entities/layout/layout.ipc'
import { activeProjectQueryOptions } from '@entities/project/project.query'
import { getSettings } from '@entities/settings/settings.ipc'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { toProblemSeverity } from '@features/problems/problem-severity'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { QUERY_KEY } from '@shared/constants/query-key'

const IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS = 300

const findFileTabByPath = (node: PaneNode, path: string): Tab | null => {
    if (node.node === 'leaf') return node.tabs.find((tab) => tab.kind.kind === 'file' && tab.kind.path === path) ?? null
    for (const child of node.children) {
        const found = findFileTabByPath(child, path)
        if (found) return found
    }
    return null
}

/**
 * Keeps the Claude Code IDE protocol (diff/save/close-tab requests, status sync, diagnostics push)
 * alive independent of any particular piece of chrome being mounted — it used to live inside
 * `StatusBarContent`, which unmounts whenever Zen mode hides the status bar (`app-shell.tsx`),
 * silently dropping the whole protocol for as long as the user stayed in that view. Mounted once at
 * the main-window app root (`app.tsx`), alongside the other provider-layer IPC sync, for the whole
 * session's lifetime.
 */
export const IdeSyncProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()
    const { data: activeProjectId = null } = useQuery(activeProjectQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: ideStatus = null } = useQuery(ideStatusQueryOptions())
    const markers = useMonacoMarkers()

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

    return children
}
