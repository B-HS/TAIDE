import type { languages } from 'monaco-editor'
import type { LspServerId, ProjectId } from '@shared/api/bindings'
import type { LspClient } from '@shared/lib/lsp/client'
import type { Monaco } from '@shared/lib/lsp/monaco-types'
import { requestDocumentSymbols } from '@shared/lib/lsp/adapters/document-symbol'
import { isCapabilityEnabled } from '@shared/lib/lsp/protocol'
import type { LeadingTrailingDebouncerScheduler } from '@shared/lib/leading-trailing-debouncer'

export type DocumentSymbolSessionWaiter<TSession> = { promise: Promise<TSession | null>; cancel: () => void }

/**
 * Trailing-only debounce window between a buffer edit and the `textDocument/documentSymbol`
 * re-request it triggers (audit §4-B B12). Trailing-only, not leading+trailing like
 * `shiki-monaco.ts`'s theme re-apply: the first keystroke of a burst is exactly when the symbol tree
 * is least worth asking for (the edit is mid-word, and the answer is superseded by the next
 * keystroke), so a leading edge would only add one guaranteed-stale round trip per burst. Long
 * enough to collapse continuous typing into a single request, short enough that the outline/
 * breadcrumb catch up before the user looks away from the edit.
 */
export const DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS = 400

const timeoutScheduler: LeadingTrailingDebouncerScheduler<ReturnType<typeof setTimeout>> = {
    schedule: (callback, delayMs) => setTimeout(callback, delayMs),
    cancel: (timerId) => clearTimeout(timerId),
}

type BuildDocumentSymbolWaitersInput<TSession> = {
    availableServerIds: LspServerId[]
    path: string
    projectId: ProjectId
    fallbackRoot: string | undefined
    isCancelled: () => boolean
    resolveRoot: (input: { serverId: LspServerId; filePath: string }) => Promise<string | null>
    waitForSession: (projectId: ProjectId, serverId: LspServerId, root: string) => DocumentSymbolSessionWaiter<TSession>
}

/**
 * root-aware conversion (`docs/acknowledge/2026-08-19-editor-pane-batch-contract.md` §1.2): resolves
 * each candidate server's actual LSP root for `path` (mirroring `use-lsp-session.ts`'s own
 * `resolveLspRoot(...) ?? projectRoot` acquire-time fallback exactly — a consumer that used a
 * different fallback could ask a root-exact session waiter for a root key nothing was ever acquired
 * under) before waiting on a session, replacing a root-agnostic waiter that could resolve to *any*
 * root's session in a multi-root project (R7#7) — including one that never had `path` open.
 * `resolveRoot`/`waitForSession`/`isCancelled` are injected rather than imported directly so this
 * decision (which roots to wait on, and in what order) is a plain, directly testable function of its
 * inputs — this project has no DOM/testing-library environment configured for `bun:test`, and the
 * only production caller is {@link loadDocumentSymbolsForPath} below (see its doc comment for who
 * calls that).
 *
 * Generic over `TSession` — rather than importing `SessionRecord` from
 * `@entities/lsp/lsp-session-registry` — so this stays in `shared` without a shared→entities
 * reference (fsd.md §2). Every real caller instantiates it implicitly: passing
 * `waitForLspSessionForRoot` (whose return type carries `SessionRecord`) as `waitForSession` lets
 * TypeScript infer `TSession = SessionRecord` at the call site.
 */
export const buildDocumentSymbolWaiters = async <TSession>({
    availableServerIds,
    path,
    projectId,
    fallbackRoot,
    isCancelled,
    resolveRoot,
    waitForSession,
}: BuildDocumentSymbolWaitersInput<TSession>): Promise<DocumentSymbolSessionWaiter<TSession>[]> => {
    const resolvedRoots = await Promise.all(availableServerIds.map((serverId) => resolveRoot({ serverId, filePath: path }).catch(() => null)))
    if (isCancelled()) return []

    return availableServerIds.flatMap((serverId, index) => {
        const root = resolvedRoots[index] ?? fallbackRoot
        return root ? [waitForSession(projectId, serverId, root)] : []
    })
}

type LoadDocumentSymbolsInput<TSession extends { ready: Promise<{ client: LspClient }> }> = Omit<
    BuildDocumentSymbolWaitersInput<TSession>,
    'isCancelled'
> & {
    monaco: Monaco
    onLoaded: (symbols: languages.DocumentSymbol[]) => void
    subscribeContentChange?: (onContentChanged: () => void) => () => void
    refreshDelayMs?: number
    scheduler?: LeadingTrailingDebouncerScheduler
}

/**
 * Combines {@link buildDocumentSymbolWaiters} with the "take the first server that comes up and
 * advertises `documentSymbolProvider`, report its `textDocument/documentSymbol` result (or `[]`
 * once every waiter is exhausted)" loop — the effect body shared verbatim by
 * `breadcrumbs-bar.tsx`/`outline-panel-container.tsx`/`use-document-symbol-loader.ts` (this
 * function's three callers, same root-aware rationale as {@link buildDocumentSymbolWaiters}'s doc
 * comment). Returns a cleanup function so a caller's `useEffect` can `return
 * loadDocumentSymbolsForPath({ ... })` directly — cancelling in-flight session waits when a newer
 * path/effect run supersedes this one.
 *
 * `subscribeContentChange` (optional) makes the result *stay* current: a document symbol tree is a
 * snapshot of the buffer at request time, and the outline/breadcrumb effects re-run only on
 * path/language/server changes, so without it a single keystroke left both surfaces describing a
 * file that no longer exists — permanently, for as long as the tab stayed open (audit §4-B B12).
 * Edits are coalesced through {@link DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS} and each re-request
 * supersedes the previous one (generation counter + waiter cancel), so a burst of typing costs one
 * `textDocument/documentSymbol` round trip, not one per change. Safe to re-request this way because
 * `use-lsp-session.ts` sends `textDocument/didChange` synchronously from the same
 * `onDidChangeContent` event — by the time this fires, the server already has the new text.
 */
export const loadDocumentSymbolsForPath = <TSession extends { ready: Promise<{ client: LspClient }> }>({
    monaco,
    availableServerIds,
    path,
    projectId,
    fallbackRoot,
    resolveRoot,
    waitForSession,
    onLoaded,
    subscribeContentChange,
    refreshDelayMs = DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS,
    scheduler = timeoutScheduler,
}: LoadDocumentSymbolsInput<TSession>): (() => void) => {
    let disposed = false
    let generation = 0
    let pendingCancels: (() => void)[] = []
    let refreshTimerId: unknown = null

    const cancelPendingWaiters = () => {
        pendingCancels.forEach((cancel) => cancel())
        pendingCancels = []
    }

    const load = async (runGeneration: number) => {
        const isStale = () => disposed || runGeneration !== generation
        const waiters = await buildDocumentSymbolWaiters({
            availableServerIds,
            path,
            projectId,
            fallbackRoot,
            isCancelled: isStale,
            resolveRoot,
            waitForSession,
        })
        if (isStale()) {
            waiters.forEach((waiter) => waiter.cancel())
            return
        }
        pendingCancels = waiters.map((waiter) => waiter.cancel)

        for (const { promise } of waiters) {
            const session = await promise
            if (!session || isStale()) continue

            const ready = await session.ready.catch(() => null)
            if (!ready || isStale()) continue
            if (!ready.client.supports((capabilities) => isCapabilityEnabled(capabilities.documentSymbolProvider))) continue

            const uri = monaco.Uri.file(path).toString()
            const result = await requestDocumentSymbols(monaco, ready.client, uri).catch(() => [])
            if (!isStale()) {
                onLoaded(result)
                return
            }
        }
        if (!isStale()) onLoaded([])
    }

    const startLoad = () => {
        cancelPendingWaiters()
        generation += 1
        void load(generation)
    }

    const clearRefreshTimer = () => {
        if (refreshTimerId === null) return
        scheduler.cancel(refreshTimerId)
        refreshTimerId = null
    }

    startLoad()

    const unsubscribeContentChange = subscribeContentChange?.(() => {
        if (disposed) return
        clearRefreshTimer()
        refreshTimerId = scheduler.schedule(() => {
            refreshTimerId = null
            if (!disposed) startLoad()
        }, refreshDelayMs)
    })

    return () => {
        disposed = true
        clearRefreshTimer()
        unsubscribeContentChange?.()
        cancelPendingWaiters()
    }
}
