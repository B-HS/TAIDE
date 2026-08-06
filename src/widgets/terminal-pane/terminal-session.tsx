import type { FC } from 'react'
import { useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import type { ProjectId, TabId } from '@shared/api/bindings'
import { currentThemeQueryOptions } from '@entities/theme/theme.query'
import { settingsQueryOptions } from '@entities/settings/settings.query'
import { terminalSessionsQueryOptions } from '@entities/terminal/terminal.query'
import { attachPty, resizePty, setPtyPaused, spawnPty, writePty } from '@entities/terminal/terminal.ipc'
import { useSetTerminalSession } from '@entities/layout/layout.query'
import { commands } from '@shared/api/bindings'
import { unwrapResult } from '@shared/api/unwrap-result'
import { toXtermTheme } from '@shared/lib/xterm-theme'
import { DEFAULT_FONT_SIZE } from '@shared/constants/terminal'
import { TerminalPane } from '@widgets/terminal-pane/terminal-pane'

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
    const { mutate: persistTerminalSession } = useSetTerminalSession(projectId)

    const isPersistedAlive = (liveSessions ?? []).some((session) => session.id === persistedSessionId)
    const sessionId = spawnedSessionId ?? (isPersistedAlive ? persistedSessionId : null)

    const spawnWithMeasuredSize = async (cols: number, rows: number) => {
        const defaults = await unwrapResult(commands.ptyDefaultOptions(projectId))
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

    const handleAttachData = (onData: (bytes: Uint8Array) => void) => {
        if (!sessionId) return () => undefined
        let active = true
        void attachPty(sessionId, (bytes) => {
            if (active) onData(bytes)
        }).catch(() => undefined)
        return () => {
            active = false
        }
    }

    if (failure) {
        return <div className='bg-terminal-background text-status-error flex h-full w-full items-center justify-center text-sm'>{failure}</div>
    }

    if (!theme || !isSessionsFetched) return <div className='bg-terminal-background h-full w-full' />

    return (
        <TerminalPane
            sessionId={sessionId}
            fontSize={settings?.terminalFontSize ?? DEFAULT_FONT_SIZE}
            theme={toXtermTheme(theme)}
            onWrite={handleWrite}
            onResize={handleResize}
            onReady={handleReady}
            onSetPaused={handleSetPaused}
            attachData={handleAttachData}
        />
    )
}
