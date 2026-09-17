import type { FC, PropsWithChildren } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { FlushScope, ProjectId } from '@shared/api/bindings'
import { events } from '@shared/api/bindings'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { HOT_EXIT_FLUSH_SAFETY_MARGIN_MS } from '@shared/constants/mirror'
import { flushMirrorsComplete } from '@entities/file/file.ipc'
import { flushAllMirrors, flushProjectMirrors } from '@entities/editor/mirror-flush-registry'
import { flushAllLspSessions } from '@entities/lsp/lsp-session-flush-registry'

/**
 * Client-side budget for how long to wait on `flushAllMirrors()` before reporting completion
 * regardless — `timeoutMs` (Rust's `HOT_EXIT_FLUSH_TIMEOUT_MS`, carried on the event so this side
 * never needs its own copy — see `events.rs::HotExitFlushRequested`'s doc comment) minus a fixed
 * safety margin for the `flushMirrorsComplete` IPC round trip itself, clamped to never go negative.
 * `timeoutMs` types as `number | null` only because every `f64` IPC field does (this project's
 * binding convention, unrelated to whether the Rust field is actually optional — it never is here);
 * `?? 0` treats a hypothetically absent value as "no budget", the conservative choice, since racing
 * against a shorter-than-intended deadline just means reporting completion sooner, never later than
 * the backend's own `HOT_EXIT_FLUSH_TIMEOUT_MS` force-close.
 */
export const computeFlushBudgetMs = (timeoutMs: number | null) => Math.max(0, (timeoutMs ?? 0) - HOT_EXIT_FLUSH_SAFETY_MARGIN_MS)

/**
 * What one window does with a flush request: write every model it holds, write one project's models,
 * or stay out of it entirely. "ignore" is the only case that also skips the confirmation — see
 * {@link planMirrorFlush}.
 */
export type MirrorFlushPlan = { target: 'ignore' } | { target: 'all' } | { target: 'project'; projectId: ProjectId }

const IGNORE_PLAN: MirrorFlushPlan = { target: 'ignore' }
const ALL_PLAN: MirrorFlushPlan = { target: 'all' }

/**
 * Decides what `scope` asks of the window whose Tauri label is `windowLabel`.
 *
 * The handshake used to be app-exit-only and broadcast, so every listener flushed everything and
 * confirmed. Two more teardowns now share it (audit wave 2 #5, #12) and `Event::emit` still reaches
 * every window regardless of the handle it was called through, so the scope has to be read here:
 *
 * - `"all"` — the app is exiting. Every window flushes everything and confirms, as before.
 * - `{ window }` — that one OS window is about to be destroyed, so only it has anything to lose.
 *   Every *other* window stays silent: `AppState::begin_flush` recorded exactly one expected label
 *   for this scope, and a confirmation from a window that was never expected would be one window
 *   answering for another's close.
 * - `{ project }` — a project is closing and its `file_mirror_dirty` writes stop being accepted the
 *   moment it leaves `AppState`. Every window is expected to confirm (`await_project_flush` collects
 *   all webview labels), including ones that render other projects and therefore flush nothing —
 *   staying silent there would hold the close open for the full timeout.
 *
 * The window axis needs no registry lookup because each OS window runs its own JS realm: everything
 * `mirror-flush-registry` holds in this window belongs to this window by construction.
 */
export const planMirrorFlush = (scope: FlushScope, windowLabel: string): MirrorFlushPlan => {
    if (scope === 'all') return ALL_PLAN
    if (scope.window !== undefined) return scope.window === windowLabel ? ALL_PLAN : IGNORE_PLAN
    return { target: 'project', projectId: scope.project }
}

const runPlannedFlush = (plan: MirrorFlushPlan) => (plan.target === 'project' ? flushProjectMirrors(plan.projectId) : flushAllMirrors())

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

/**
 * Answers the Rust close/teardown interceptions: the backend defers whatever was about to destroy a
 * webview or drop a project's state, emits `HotExitFlushRequested` carrying the {@link FlushScope} it
 * is waiting on, and this provider pushes the matching editor panes' pending hot-exit mirror writes
 * (registered via `mirror-flush-registry`) to disk before confirming with that same scope. Races that
 * flush against {@link computeFlushBudgetMs}'s client-side deadline (`Promise.race`, not
 * `Promise.any` — a settled `sleep` still lets the flush keep running for whichever panes haven't
 * finished) so one pane stuck on an IPC call that never settles can't silently turn a normal
 * handshake into the backend's own `HOT_EXIT_FLUSH_TIMEOUT_MS` force-close, which would also lose
 * whatever *other* panes had already finished flushing by then. A backend-side timeout still proceeds
 * without this window if it never reports back at all (e.g. the IPC call to confirm itself fails), so
 * a swallowed failure here can't hang the app either way.
 *
 * LSP sessions still sitting in their dispose grace period are flushed only when this window is the
 * one being torn down ({@link flushAllLspSessions}, best-effort and unraced against the mirror budget,
 * since `LspStore::kill_all` at `RunEvent::Exit` kills every language server regardless of whether
 * this handshake completes in time). A project close is deliberately not one of those cases: it has
 * its own per-project disposal path through `ipc-sync-provider.tsx`'s `projectClosed` handler, and
 * flushing *every* session there would tear down servers belonging to projects that are staying open.
 * Reached through `lsp-session-flush-registry` rather than `lsp-session-registry` directly so this
 * otherwise monaco-free provider (and its unit test) doesn't drag that module's real monaco worker
 * imports into every module graph that touches this file.
 */
export const HotExitFlushProvider: FC<PropsWithChildren> = ({ children }) => {
    useTauriEvent(events.appHotExitFlushRequested, ({ payload }) => {
        const plan = planMirrorFlush(payload.scope, getCurrentWindow().label)
        if (plan.target === 'ignore') return
        if (plan.target === 'all') flushAllLspSessions()
        const budgetMs = computeFlushBudgetMs(payload.timeoutMs)
        void Promise.race([runPlannedFlush(plan), sleep(budgetMs)]).finally(() => void flushMirrorsComplete(payload.scope).catch(() => undefined))
    })

    return children
}
