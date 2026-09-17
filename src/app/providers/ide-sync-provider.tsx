import type { FC, PropsWithChildren } from 'react'
import { useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { OpenedFile, ProjectLayout } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { getModel } from '@entities/editor/model-registry'
import { useSaveFile } from '@entities/file/file.query'
import { removePendingClaudeDiff, setPendingClaudeDiff } from '@entities/ide/claude-diff-registry'
import { publishIdeDiagnostics, resolveIdeDiff, resolveIdeSave } from '@entities/ide/ide.ipc'
import { ideStatusQueryOptions, useIdeStatusSync } from '@entities/ide/ide.query'
import { setTabDirty } from '@entities/layout/layout.ipc'
import { useOpenTabInProject } from '@entities/layout/layout.query'
import { getSettings } from '@entities/settings/settings.ipc'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { useFocusedProjectId } from '@app/providers/shell-slot-provider'
import { toProblemSeverity } from '@features/problems/problem-severity'
import { useMonacoMarkers } from '@shared/hooks/use-monaco-markers'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { monacoRangeToLsp } from '@shared/lib/lsp/position'
import { collectAllPaneTabs } from '@shared/lib/pane-tree'
import { QUERY_KEY } from '@shared/constants/query-key'

const IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS = 300

/**
 * Keeps the Claude Code IDE protocol (diff/save/close-tab requests, status sync, diagnostics push)
 * alive independent of any particular piece of chrome being mounted — it used to live inside
 * `StatusBarContent`, which unmounts whenever Zen mode hides the status bar (`app-shell.tsx`),
 * silently dropping the whole protocol for as long as the user stayed in that view. Mounted once at
 * the main-window app root (`app.tsx`), alongside the other provider-layer IPC sync, for the whole
 * session's lifetime.
 *
 * The diagnostics push is scoped to the *focused shell slot's* project (d-62 §1.B): with several
 * project shells open side by side, the markers monaco reports belong to the shell the user is
 * working in, and attributing them to a different project's IDE session would be wrong rather than
 * merely stale.
 */
export const IdeSyncProvider: FC<PropsWithChildren> = ({ children }) => {
    const queryClient = useQueryClient()
    const activeProjectId = useFocusedProjectId()
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: ideStatus = null } = useQuery(ideStatusQueryOptions())
    const markers = useMonacoMarkers()
    const { mutateAsync: openTabInProject } = useOpenTabInProject()
    const { mutateAsync: saveFileMutation } = useSaveFile()

    useTauriEvent(events.ideDiffRequested, ({ payload }) => {
        setPendingClaudeDiff(payload.requestId, { oldPath: payload.oldPath, newContents: payload.newContents, tabName: payload.tabName })

        void (async () => {
            const current = settings ?? (await getSettings())
            if (!current.ideAutoOpenDiff) {
                removePendingClaudeDiff(payload.requestId)
                await resolveIdeDiff({ requestId: payload.requestId, outcome: 'rejected', content: null })
                return
            }

            await openTabInProject({
                projectId: payload.projectId,
                kind: { kind: 'claudeDiff', requestId: payload.requestId, path: payload.newPath },
                title: payload.tabName,
                target: null,
                preview: false,
            })
        })().catch((error: unknown) => toast.error(describeIpcError(error)))
    })

    useTauriEvent(events.ideSaveRequested, ({ payload }) => {
        void (async () => {
            const layout = queryClient.getQueryData<ProjectLayout>(QUERY_KEY.LAYOUT.DETAIL(payload.projectId))
            /**
             * Every tree in the project, not `layout.root` alone: `move_tab_to_new_window` *removes*
             * the tab from the main tree, so a file being edited in an auxiliary window is only ever
             * found through `collectAllPaneTabs`. Missing it made `!tab?.dirty` true and answered the
             * agent "Document saved successfully" without writing a byte (audit #2). Actually saving
             * a tab that lives in another window is still out of reach from here — this provider is
             * main-window-only and `model-registry` is per-realm — so that case now resolves
             * `saved: false` through the `!model` branch below instead of lying.
             */
            const tab = layout
                ? (collectAllPaneTabs(layout).find((open) => open.kind.kind === 'file' && open.kind.path === payload.path) ?? null)
                : null

            if (!tab?.dirty) {
                await resolveIdeSave({ requestId: payload.requestId, saved: true }).catch(() => undefined)
                return
            }

            const model = getModel(payload.path)
            if (!model) {
                await resolveIdeSave({ requestId: payload.requestId, saved: false }).catch(() => undefined)
                return
            }

            /**
             * A read-only file is never written back from here either (the editor's own save path
             * refuses it in `use-editor-file-persistence.ts`'s `handleSave`). It can still be dirty
             * without ever having been typed into — a hot-exit mirror restore or a background
             * `WorkspaceEdit` both install a draft without going through the editor — and for a file
             * forced read-only by `encodingLossy` writing that draft back would burn its U+FFFD
             * replacements into the original bytes permanently (audit §4-A-3).
             */
            if (queryClient.getQueryData<OpenedFile>(QUERY_KEY.FILE.CONTENT(payload.path))?.readOnly) {
                await resolveIdeSave({ requestId: payload.requestId, saved: false }).catch(() => undefined)
                return
            }

            try {
                /**
                 * `useSaveFile` runs without a `projectId` here (this provider has no single project
                 * context — the request carries its own), so its own `onSuccess` cannot invalidate the
                 * project-scoped caches this write invalidates. `FILE.MIRRORS` is the one that matters:
                 * Rust's `file_save` discards the hot-exit mirror for `path`, but the cached mirror list
                 * is `staleTime: Infinity`, so without this the pre-save entry stays in the cache and is
                 * "restored" over an already-clean file the next time a pane mounts on that path. The
                 * *mounted* panes settle through `file-save-settle-registry` (published by `useSaveFile`
                 * itself) — this covers the tabs that had no pane rendered at save time.
                 */
                await saveFileMutation({ path: payload.path, content: model.getValue() })
                void queryClient.invalidateQueries({ queryKey: QUERY_KEY.FILE.MIRRORS(payload.projectId) })
                await setTabDirty({ tabId: tab.id, dirty: false })
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
            void publishIdeDiagnostics({ projectId: activeProjectId, items }).catch(() => undefined)
        }, IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS)

        return () => clearTimeout(timeout)
    }, [markers, activeProjectId, ideStatus?.running])

    return children
}
