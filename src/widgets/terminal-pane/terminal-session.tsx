import type { FC, RefObject } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { PaneId, ProjectId, TabId, TerminalSession as TerminalSessionInfo } from '@shared/api/bindings'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { projectQueryOptions } from '@entities/project/project.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { terminalSessionsQueryOptions } from '@entities/terminal/terminal.query'
import { isTerminalSessionAlive, removeTerminalSession, upsertTerminalSession } from '@entities/terminal/terminal-session-cache'
import { attachPty, detachPty, killPty, resizePty, resolveTerminalPath, setPtyPaused, spawnPty, writePty } from '@entities/terminal/terminal.ipc'
import { openExternalUrl } from '@entities/system/external-url'
import { layoutQueryOptions, useCloseTab, useOpenTab, useOpenTabInSplit, useSetTerminalSession } from '@entities/layout/layout.query'
import { commands, events } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'
import { toXtermTheme } from '@shared/lib/xterm-theme'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { findPaneTab } from '@shared/lib/pane-tree'
import { registerTerminalWriteHandler } from '@shared/lib/bridge/terminal-write-bridge'
import { requestOpenFileFromEditor } from '@shared/lib/bridge/editor-opener-bridge'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import { useTauriEvent } from '@shared/hooks/use-tauri-event'
import { useIpcErrorMessage } from '@shared/hooks/use-ipc-error-message'
import { DEFAULT_RESIZER_THICKNESS } from '@shared/constants/layout'
import { DEFAULT_FONT_SIZE, DEFAULT_SCROLLBACK, DEFAULT_SHELL_LABEL } from '@shared/constants/terminal'
import { QUERY_KEY } from '@shared/constants/query-key'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import type { TerminalCursorStyle } from '@features/terminal/terminal-view'
import { normalizeDecorationHexColor } from '@features/terminal/terminal-osc133'
import { Button } from '@shared/ui/button'
import { TerminalPane } from '@widgets/terminal-pane/terminal-pane'
import { appendPendingTerminalInput } from '@widgets/terminal-pane/pending-terminal-input'
import { resolveSplitTerminalCwd } from '@widgets/terminal-pane/terminal-split-availability'

const DEFAULT_TERMINAL_CURSOR_STYLE: TerminalCursorStyle = 'bar'

type TerminalSessionProps = {
    projectId: ProjectId
    tabId: TabId
    /** The pane this terminal is rendered in — the split target of its context menu. */
    paneId: PaneId
    sessionId: string
    /** Whether this terminal's pane is the focused one — see `TerminalViewProps.autoFocus`. */
    autoFocus: boolean
    /** The pane's terminal area, measured on context-menu open — see `TerminalPaneProps.paneElementRef`. */
    paneElementRef: RefObject<HTMLDivElement | null>
}

export const TerminalSession: FC<TerminalSessionProps> = ({ projectId, tabId, paneId, sessionId: persistedSessionId, autoFocus, paneElementRef }) => {
    const spawnStartedRef = useRef(false)
    const dimensionsRef = useRef({ cols: 0, rows: 0 })
    const isMountedRef = useRef(true)
    const pendingInputRef = useRef('')

    const [spawnedSessionId, setSpawnedSessionId] = useState<string | null>(null)
    const [failure, setFailure] = useState<unknown>(null)
    const [cwd, setCwd] = useState<string | null>(null)
    const [exited, setExited] = useState<{ code: number | null } | null>(null)

    const { data: theme } = useQuery(currentThemeQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: project } = useQuery(projectQueryOptions(projectId))
    const { data: liveSessions, isFetched: isSessionsFetched } = useQuery(terminalSessionsQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutateAsync: persistTerminalSession } = useSetTerminalSession(projectId)
    const { mutate: openTabInSplit } = useOpenTabInSplit(projectId)
    const { mutate: openTab } = useOpenTab(projectId)
    const { mutate: closeTab } = useCloseTab(projectId)
    const { t } = useTranslation()
    const queryClient = useQueryClient()
    const failureMessage = useIpcErrorMessage(failure)

    const persistedSession = (liveSessions ?? []).find((session) => session.id === persistedSessionId)
    const sessionId = exited ? null : (spawnedSessionId ?? (isTerminalSessionAlive(liveSessions, persistedSessionId) ? persistedSessionId : null))
    const activeTabKind = layout ? findPaneTab(layout.root, tabId)?.kind : null
    const tabCwd = activeTabKind?.kind === 'terminal' ? (activeTabKind.cwd ?? null) : null

    const flushPendingInput = (created: string) => {
        const pending = pendingInputRef.current
        pendingInputRef.current = ''
        if (!pending) return
        void writePty({ sessionId: created, data: pending }).catch(() => undefined)
    }

    /**
     * Records the spawned session on its tab and reports whether anything still owns it.
     *
     * `layout_set_terminal_session` fails with `NotFound` when the tab no longer exists, which is
     * exactly how a tab closed *while its first spawn was still in flight* announces itself: the
     * spawn holds the app-wide mutation guard, so the close command cannot even run until the pty is
     * already alive, and it then kills the `sessionId` recorded on the tab — which is still empty.
     * The shell would survive with nothing referencing it, invisible until app exit (audit §4-B
     * C14). Distinguishing that from an ordinary tab *switch* (which unmounts this component just
     * the same, and must keep its session for the next visit) is what the persist result is read
     * for: a switched-away tab still exists, so its persist succeeds. A persist that fails while
     * this component is still mounted leaves a working terminal on screen and is left alone, as
     * before.
     */
    const settleSpawnedSession = async (created: string) => {
        try {
            await persistTerminalSession({ tabId, sessionId: created })
            return true
        } catch {
            if (isMountedRef.current) return true
            await killPty(created).catch(() => undefined)
            queryClient.setQueryData<TerminalSessionInfo[]>(QUERY_KEY.TERMINAL.SESSIONS(projectId), (sessions) =>
                removeTerminalSession(sessions, created),
            )
            return false
        }
    }

    const spawnWithMeasuredSize = async (cols: number, rows: number) => {
        const defaults = await unwrapResult(commands.ptyDefaultOptions(projectId, tabCwd))
        const created = await spawnPty({ ...defaults, cols, rows }, () => undefined)
        setSpawnedSessionId(created)
        setCwd(defaults.cwd)
        /**
         * `terminalSessionsQueryOptions` is `staleTime: Infinity`, so this write — not a refetch — is
         * what makes the session this tab just spawned visible to its own next mount. Without it the
         * roster still predated the spawn, `isTerminalSessionAlive` read the persisted id as dead,
         * and every return to this tab spawned another shell while orphaning the previous one (audit
         * §4-B A6, `docs/features/terminal.md` §3.1).
         */
        queryClient.setQueryData<TerminalSessionInfo[]>(QUERY_KEY.TERMINAL.SESSIONS(projectId), (sessions) =>
            upsertTerminalSession(sessions, {
                id: created,
                projectId,
                cwd: defaults.cwd,
                shell: defaults.shell ?? DEFAULT_SHELL_LABEL,
                running: true,
            }),
        )

        if (!(await settleSpawnedSession(created))) return

        const latest = dimensionsRef.current
        if (latest.cols !== cols || latest.rows !== rows) await resizePty({ sessionId: created, cols: latest.cols, rows: latest.rows })
        flushPendingInput(created)
    }

    const handleSpawnFailure = (error: unknown) => {
        spawnStartedRef.current = false
        pendingInputRef.current = ''
        setFailure(error)
        toast.error(describeIpcError(error))
    }

    const handleReady = (cols: number, rows: number) => {
        dimensionsRef.current = { cols, rows }
        if (sessionId) {
            void resizePty({ sessionId, cols, rows }).catch(() => undefined)
            return
        }
        if (spawnStartedRef.current) return
        spawnStartedRef.current = true
        void spawnWithMeasuredSize(cols, rows).catch(handleSpawnFailure)
    }

    const handleResize = (cols: number, rows: number) => {
        dimensionsRef.current = { cols, rows }
        if (!sessionId) return
        void resizePty({ sessionId, cols, rows }).catch(() => undefined)
    }

    /**
     * Input typed before the first spawn resolves is buffered instead of dropped (audit §4-B C14) —
     * a terminal tab measures itself, spawns, and only then has a session to write to, and anything
     * typed in that window used to vanish with no echo of any kind. Buffering is gated on a spawn
     * actually being in flight: with no session and no spawn there is nothing that will ever flush.
     */
    const handleWrite = (data: string) => {
        if (!sessionId) {
            if (spawnStartedRef.current) pendingInputRef.current = appendPendingTerminalInput(pendingInputRef.current, data)
            return
        }
        void writePty({ sessionId, data }).catch(() => undefined)
    }

    const handleSetPaused = (paused: boolean) => {
        if (!sessionId) return
        void setPtyPaused({ sessionId, paused }).catch(() => undefined)
    }

    const handleOpenLink = (uri: string) => {
        void openExternalUrl(uri).catch(() => toast.error(t('terminal.openLinkFailed')))
    }

    const handleOpenFileLink = (match: TerminalLinkMatch) => {
        const effectiveCwd = cwd ?? persistedSession?.cwd ?? tabCwd
        if (!effectiveCwd) return
        void resolveTerminalPath({ path: match.path, cwd: effectiveCwd })
            .then((resolvedPath) => requestOpenFileFromEditor({ path: resolvedPath, line: match.line ?? 1, column: match.column ?? 1 }))
            .catch(() => toast.error(t('terminal.openLinkFailed')))
    }

    const notifyError = (error: Error) => toast.error(describeIpcError(error))

    /**
     * The context menu's "Split" — a *new* terminal in a *new* pane beside this one, VS Code's
     * meaning of the word, which is why it goes through `layout_open_tab_in_split` rather than the
     * `layout_split` the tab bar uses (that one moves this very tab, leaving the original pane
     * empty). Composing the two frontend-side would open the new tab in *this* pane first, tearing
     * down the terminal the user is looking at and spawning its shell twice; the single command
     * makes that unrepresentable. See `docs/features/terminal.md` §6.2.
     */
    const handleSplitNewTerminal = (edge: SplitEdge) =>
        openTabInSplit(
            {
                projectId,
                targetPane: paneId,
                edge,
                kind: {
                    kind: 'terminal',
                    sessionId: '',
                    cwd: resolveSplitTerminalCwd({
                        liveCwd: cwd,
                        persistedCwd: persistedSession?.cwd ?? null,
                        tabCwd,
                        projectRoot: project?.root ?? null,
                    }),
                },
                title: t('terminal.title'),
                preview: false,
            },
            { onError: notifyError },
        )

    const handleNewTerminal = () =>
        openTab(
            { projectId, kind: { kind: 'terminal', sessionId: '' }, title: t('terminal.title'), target: paneId, preview: false },
            { onError: notifyError },
        )

    const handleKillTerminal = () => closeTab(tabId, { onError: notifyError })

    const handleRestart = () => {
        setExited(null)
        setSpawnedSessionId(null)
        spawnStartedRef.current = true
        const { cols, rows } = dimensionsRef.current
        void spawnWithMeasuredSize(cols, rows).catch(handleSpawnFailure)
    }

    const handleAttachData = (onData: (bytes: Uint8Array) => void) => {
        if (!sessionId) return () => undefined
        const activeSessionId = sessionId
        let active = true
        let subscriptionId: number | null = null
        void attachPty(activeSessionId, (bytes) => {
            if (active) onData(bytes)
        })
            .then((resolvedSubscriptionId) => {
                if (!active) {
                    void detachPty(activeSessionId, resolvedSubscriptionId).catch(() => undefined)
                    return
                }
                subscriptionId = resolvedSubscriptionId
            })
            .catch(() => undefined)
        return () => {
            active = false
            if (subscriptionId !== null) void detachPty(activeSessionId, subscriptionId).catch(() => undefined)
        }
    }

    const handleTerminalWriteRequest = useEffectEvent((data: string) => handleWrite(data))

    /**
     * Exposes this tab's pty input path (`handleWrite`, via the non-reactive `useEffectEvent`
     * wrapper so the effect doesn't need to re-subscribe on every render) to `terminal-write-bridge`
     * so cross-widget callers — "Run Selected Text in Terminal", the task runner — can write into
     * this session without holding a reference to it. Registered only once `sessionId` is live so
     * writes that arrive before the first spawn resolves are queued (see the bridge's own doc)
     * rather than silently dropped.
     */
    useEffect(() => {
        if (!sessionId) return
        return registerTerminalWriteHandler(tabId, handleTerminalWriteRequest)
    }, [tabId, sessionId])

    /**
     * Tracks whether this pane is still on screen when an in-flight spawn finally resolves — the one
     * thing `settleSpawnedSession` cannot learn from the IPC layer alone. Re-armed on mount rather
     * than only cleared on unmount because `StrictMode`'s mount→cleanup→remount replay reuses this
     * same ref (`main.tsx`).
     */
    useEffect(() => {
        isMountedRef.current = true
        return () => {
            isMountedRef.current = false
        }
    }, [])

    useTauriEvent(events.terminalCwdChanged, ({ payload }) => {
        if (payload.sessionId !== sessionId) return
        setCwd(payload.cwd)
    })

    /**
     * Only this pane's own "[process exited]" screen is decided here. Marking the dead session in the
     * `TERMINAL.SESSIONS` roster is `ipc-sync-provider.tsx`'s job (audit §4-B B14): a session whose
     * tab is in the background has no mounted component to hear its exit, so that bookkeeping cannot
     * live behind this self-session guard.
     */
    useTauriEvent(events.terminalExited, ({ payload }) => {
        if (payload.sessionId !== sessionId) return
        setExited({ code: payload.code })
    })

    if (failure) {
        return <div className='bg-terminal-background text-status-error flex h-full w-full items-center justify-center text-sm'>{failureMessage}</div>
    }

    if (exited) {
        return (
            <div className='bg-terminal-background text-muted-foreground flex h-full w-full flex-col items-center justify-center gap-2 text-sm'>
                <span>{exited.code === null ? t('terminal.processExited') : `${t('terminal.processExited')} (${exited.code})`}</span>
                <Button size='sm' variant='outline' onClick={handleRestart}>
                    {t('terminal.restart')}
                </Button>
            </div>
        )
    }

    if (!theme || !isSessionsFetched) return <div className='bg-terminal-background h-full w-full' />

    return (
        <TerminalPane
            sessionId={sessionId}
            autoFocus={autoFocus}
            fontSize={settings?.terminalFontSize ?? DEFAULT_FONT_SIZE}
            fontFamily={buildMonospaceFontStack(settings?.terminalFontFamily ?? null)}
            theme={toXtermTheme(theme)}
            scrollback={settings?.terminalScrollback ?? DEFAULT_SCROLLBACK}
            cursorStyle={settings?.terminalCursorStyle ?? DEFAULT_TERMINAL_CURSOR_STYLE}
            cursorBlink={settings?.terminalCursorBlink ?? true}
            commandSuccessColor={normalizeDecorationHexColor(theme.colors['statusIndicator.success'])}
            commandFailureColor={normalizeDecorationHexColor(theme.colors['statusIndicator.error'])}
            paneElementRef={paneElementRef}
            resizerThicknessPx={settings?.resizerThickness ?? DEFAULT_RESIZER_THICKNESS}
            onWrite={handleWrite}
            onResize={handleResize}
            onReady={handleReady}
            onSetPaused={handleSetPaused}
            onOpenLink={handleOpenLink}
            onOpenFileLink={handleOpenFileLink}
            onSplitNewTerminal={handleSplitNewTerminal}
            onNewTerminal={handleNewTerminal}
            onKillTerminal={handleKillTerminal}
            attachData={handleAttachData}
        />
    )
}
