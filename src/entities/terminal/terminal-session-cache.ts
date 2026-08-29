import type { TerminalSession } from '@shared/api/bindings'

/**
 * `terminalSessionsQueryOptions` is `staleTime: Infinity` — it is fetched once per project and then
 * never refetched on its own, so anything that changes the live pty roster has to write into that
 * cache or the roster silently drifts from the backend for the rest of the session. These three
 * updaters are that write path, kept pure (and returning `undefined` for "nothing changed", which
 * TanStack Query's `setQueryData` treats as a no-op) so no call site has to hand-roll list surgery:
 *
 * - {@link upsertTerminalSession} — a freshly spawned session (`terminal-session.tsx`). Without it a
 *   terminal tab that spawned a shell and was then switched away from came back to a roster that
 *   never heard of its own session, decided the persisted id was dead, and spawned *another* shell,
 *   orphaning the first one on every visit (audit §4-B A6, `docs/features/terminal.md` §3).
 * - {@link markTerminalSessionExited} — a `terminal:exited` event (`ipc-sync-provider.tsx`). This one
 *   has to run window-wide rather than inside the dying session's own pane: the pane that owns a
 *   session is unmounted while its tab is in the background (`pane-node-view.tsx` renders only the
 *   active tab), so an exit that happens then had no listener at all and left `running: true` in the
 *   cache forever — returning to that tab attached to a dead pty and showed a terminal that accepted
 *   input and answered nothing (audit §4-B B14).
 * - {@link removeTerminalSession} — a session killed before anything could own it (a terminal tab
 *   closed while its spawn was still in flight).
 *
 * A `sessions` of `undefined` (the query was never fetched in this window) is left alone rather than
 * seeded with a one-element list: a partial roster written into an `Infinity`-staleTime query would
 * be taken for the whole truth and never corrected.
 */
export const upsertTerminalSession = (sessions: TerminalSession[] | undefined, session: TerminalSession) => {
    if (!sessions) return undefined
    const index = sessions.findIndex((candidate) => candidate.id === session.id)
    if (index === -1) return [...sessions, session]
    return sessions.map((candidate) => (candidate.id === session.id ? session : candidate))
}

export const markTerminalSessionExited = (sessions: TerminalSession[] | undefined, sessionId: string) => {
    if (!sessions) return undefined
    if (!sessions.some((session) => session.id === sessionId && session.running)) return undefined
    return sessions.map((session) => (session.id === sessionId ? { ...session, running: false } : session))
}

/**
 * Whether the roster says `sessionId` is still a running pty — the decision a remounting terminal
 * tab makes between re-attaching to the session its tab already owns and spawning a brand new shell.
 * An unfetched roster reads as "not alive", the same as a missing entry: a tab cannot attach to a
 * session it has no evidence for.
 */
export const isTerminalSessionAlive = (sessions: TerminalSession[] | undefined, sessionId: string) =>
    sessions?.some((session) => session.id === sessionId && session.running) ?? false

export const removeTerminalSession = (sessions: TerminalSession[] | undefined, sessionId: string) => {
    if (!sessions) return undefined
    if (!sessions.some((session) => session.id === sessionId)) return undefined
    return sessions.filter((session) => session.id !== sessionId)
}
