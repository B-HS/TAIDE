import type { FC } from 'react'
import { useEffect, useEffectEvent, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { terminalSessionsQueryOptions } from '@entities/terminal/terminal.query'
import { attachPty, detachPty, resizePty, setPtyPaused, spawnPty, writePty } from '@entities/terminal/terminal.ipc'
import { systemOpenExternalUrl } from '@entities/system/system.ipc'
import { layoutQueryOptions, useSetTerminalSession } from '@entities/layout/layout.query'
import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'
import { toXtermTheme } from '@shared/lib/xterm-theme'
import { buildMonospaceFontStack } from '@shared/lib/font-stack'
import { findPaneTab } from '@shared/lib/pane-tree'
import { registerTerminalWriteHandler } from '@shared/lib/terminal-write-bridge'
import { DEFAULT_FONT_SIZE, DEFAULT_SCROLLBACK } from '@shared/constants/terminal'
import type { TerminalCursorStyle } from '@features/terminal/terminal-view'
import { normalizeDecorationHexColor } from '@features/terminal/terminal-osc133'
import { TerminalPane } from '@widgets/terminal-pane/terminal-pane'
import { openTerminalLink, openViaBrowserWindow } from '@widgets/terminal-pane/terminal-link-opener'

const DEFAULT_TERMINAL_CURSOR_STYLE: TerminalCursorStyle = 'bar'

type TerminalSessionProps = {
    projectId: ProjectId
    tabId: TabId
    sessionId: string
}

export const TerminalSession: FC<TerminalSessionProps> = ({ projectId, tabId, sessionId: persistedSessionId }) => {
    const spawnStartedRef = useRef(false)
    const dimensionsRef = useRef({ cols: 0, rows: 0 })

    const [spawnedSessionId, setSpawnedSessionId] = useState<string | null>(null)
    const [failure, setFailure] = useState<string | null>(null)

    const { data: theme } = useQuery(currentThemeQueryOptions())
    const { data: settings } = useQuery(settingsQueryOptions())
    const { data: liveSessions, isFetched: isSessionsFetched } = useQuery(terminalSessionsQueryOptions(projectId))
    const { data: layout } = useQuery(layoutQueryOptions(projectId))
    const { mutate: persistTerminalSession } = useSetTerminalSession(projectId)
    const { t } = useTranslation()

    const isPersistedAlive = (liveSessions ?? []).some((session) => session.id === persistedSessionId)
    const sessionId = spawnedSessionId ?? (isPersistedAlive ? persistedSessionId : null)
    const activeTabKind = layout ? findPaneTab(layout.root, tabId)?.kind : null
    const tabCwd = activeTabKind?.kind === 'terminal' ? (activeTabKind.cwd ?? null) : null

    const spawnWithMeasuredSize = async (cols: number, rows: number) => {
        const defaults = await unwrapResult(commands.ptyDefaultOptions(projectId, tabCwd))
        const created = await spawnPty({ ...defaults, cols, rows }, () => undefined)
        setSpawnedSessionId(created)
        persistTerminalSession({ tabId, sessionId: created })
        const latest = dimensionsRef.current
        if (latest.cols !== cols || latest.rows !== rows) await resizePty({ sessionId: created, cols: latest.cols, rows: latest.rows })
    }

    const handleReady = (cols: number, rows: number) => {
        dimensionsRef.current = { cols, rows }
        if (sessionId) {
            void resizePty({ sessionId, cols, rows }).catch(() => undefined)
            return
        }
        if (spawnStartedRef.current) return
        spawnStartedRef.current = true
        void spawnWithMeasuredSize(cols, rows).catch((error: Error) => {
            spawnStartedRef.current = false
            setFailure(error.message)
            toast.error(error.message)
        })
    }

    const handleResize = (cols: number, rows: number) => {
        dimensionsRef.current = { cols, rows }
        if (!sessionId) return
        void resizePty({ sessionId, cols, rows }).catch(() => undefined)
    }

    const handleWrite = (data: string) => {
        if (!sessionId) return
        void writePty({ sessionId, data }).catch(() => undefined)
    }

    const handleSetPaused = (paused: boolean) => {
        if (!sessionId) return
        void setPtyPaused({ sessionId, paused }).catch(() => undefined)
    }

    const handleOpenLink = (uri: string) => {
        void openTerminalLink(uri, {
            windowOpen: (target) => openViaBrowserWindow(target, () => window.open()),
            openExternalUrl: systemOpenExternalUrl,
        }).catch(() => toast.error(t('terminal.openLinkFailed')))
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

    if (failure) {
        return <div className='bg-terminal-background text-status-error flex h-full w-full items-center justify-center text-sm'>{failure}</div>
    }

    if (!theme || !isSessionsFetched) return <div className='bg-terminal-background h-full w-full' />

    return (
        <TerminalPane
            sessionId={sessionId}
            fontSize={settings?.terminalFontSize ?? DEFAULT_FONT_SIZE}
            fontFamily={buildMonospaceFontStack(settings?.terminalFontFamily ?? null)}
            theme={toXtermTheme(theme)}
            scrollback={settings?.terminalScrollback ?? DEFAULT_SCROLLBACK}
            cursorStyle={settings?.terminalCursorStyle ?? DEFAULT_TERMINAL_CURSOR_STYLE}
            cursorBlink={settings?.terminalCursorBlink ?? true}
            commandSuccessColor={normalizeDecorationHexColor(theme.colors['statusIndicator.success'])}
            commandFailureColor={normalizeDecorationHexColor(theme.colors['statusIndicator.error'])}
            onWrite={handleWrite}
            onResize={handleResize}
            onReady={handleReady}
            onSetPaused={handleSetPaused}
            onOpenLink={handleOpenLink}
            attachData={handleAttachData}
        />
    )
}
