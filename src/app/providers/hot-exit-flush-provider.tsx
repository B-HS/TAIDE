import type { FC, PropsWithChildren } from 'react'
import { events } from '@shared/api/bindings'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { HOT_EXIT_FLUSH_SAFETY_MARGIN_MS } from '@shared/constants/mirror'
import { flushMirrorsComplete } from '@entities/file/file.ipc'
import { flushAllMirrors } from '@entities/editor/mirror-flush-registry'
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

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))

/**
 * Answers the Rust `CloseRequested` interception: the backend defers the actual window close and
 * emits `HotExitFlushRequested`, this provider pushes every currently mounted editor pane's
 * pending hot-exit mirror write (registered via `mirror-flush-registry`) to disk, then confirms
 * completion so the app can resume exiting. Races that flush against {@link computeFlushBudgetMs}'s
 * client-side deadline (`Promise.race`, not `Promise.any` — a settled `sleep` still lets
 * `flushAllMirrors()` keep running for whichever panes haven't finished) so one pane stuck on an
 * IPC call that never settles can't silently turn a normal handshake exit into the backend's own
 * `HOT_EXIT_FLUSH_TIMEOUT_MS` force-close, which would also lose whatever *other* panes had already
 * finished flushing by then. A backend-side timeout still force-closes the window if this never
 * reports back at all (e.g. the IPC call to confirm itself fails), so a swallowed failure here
 * can't hang the app either way. Also flushes every LSP session still sitting in its dispose grace
 * period ({@link flushAllLspSessions}) — best-effort and unraced against the mirror budget, since
 * `LspStore::kill_all` (Rust, `RunEvent::Exit`) unconditionally kills every language server process
 * regardless of whether this handshake completes in time. Reached through
 * `lsp-session-flush-registry` rather than `lsp-session-registry` directly so this otherwise
 * monaco-free provider (and its `computeFlushBudgetMs` unit test) doesn't drag that module's real
 * monaco worker imports into every module graph that touches this file.
 */
export const HotExitFlushProvider: FC<PropsWithChildren> = ({ children }) => {
    useTauriEvent(events.appHotExitFlushRequested, ({ payload }) => {
        flushAllLspSessions()
        const budgetMs = computeFlushBudgetMs(payload.timeoutMs)
        void Promise.race([flushAllMirrors(), sleep(budgetMs)]).finally(() => void flushMirrorsComplete().catch(() => undefined))
    })

    return children
}
