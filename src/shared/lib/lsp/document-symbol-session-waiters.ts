import type { LspServerId, ProjectId } from '@shared/api/bindings'

export type DocumentSymbolSessionWaiter<TSession> = { promise: Promise<TSession | null>; cancel: () => void }

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
 * inputs — none of `breadcrumbs-bar.tsx`/`outline-panel-container.tsx`/`command-palette.tsx` (this
 * function's three callers) have a render-test harness to reach for (no DOM/testing-library
 * environment configured for `bun:test` in this project).
 *
 * Generic over `TSession` — rather than importing `SessionRecord` from
 * `@widgets/editor-pane/lsp-session-registry` — so this stays in `shared` without a shared→widgets
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
