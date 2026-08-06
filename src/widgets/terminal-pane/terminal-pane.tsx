import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import type { ITheme } from '@xterm/xterm'
import { TerminalView } from '@features/terminal/terminal-view'
import type { TerminalAttachHandle } from '@features/terminal/terminal-view'
import { INITIAL_FLOW_CONTROL_STATE, evaluateFlowControl, shouldTogglePause } from '@widgets/terminal-pane/terminal-flow-control'

export type TerminalPaneProps = {
    sessionId: string | null
    fontSize: number
    theme: ITheme
    onWrite: (data: string) => void
    onResize: (cols: number, rows: number) => void
    onReady: (cols: number, rows: number) => void
    onSetPaused: (paused: boolean) => void
    attachData: (onData: (bytes: Uint8Array) => void) => () => void
}

export const TerminalPane: FC<TerminalPaneProps> = ({ sessionId, fontSize, theme, onWrite, onResize, onReady, onSetPaused, attachData }) => {
    const attachRef = useRef<TerminalAttachHandle | null>(null)
    const flowStateRef = useRef(INITIAL_FLOW_CONTROL_STATE)
    const onWriteRef = useRef(onWrite)
    const onResizeRef = useRef(onResize)
    const onReadyRef = useRef(onReady)
    const onSetPausedRef = useRef(onSetPaused)
    const attachDataRef = useRef(attachData)

    useEffect(() => {
        onWriteRef.current = onWrite
        onResizeRef.current = onResize
        onReadyRef.current = onReady
        onSetPausedRef.current = onSetPaused
        attachDataRef.current = attachData
    })

    const handleData = (data: string) => onWriteRef.current(data)

    const handleResize = (cols: number, rows: number) => onResizeRef.current(cols, rows)

    const handleReady = (cols: number, rows: number) => onReadyRef.current(cols, rows)

    const handleWriteBacklogChange = (pendingBytes: number) => {
        const next = evaluateFlowControl(flowStateRef.current, pendingBytes)
        if (shouldTogglePause(flowStateRef.current, next)) onSetPausedRef.current(next.paused)
        flowStateRef.current = next
    }

    useEffect(() => {
        if (!sessionId) return
        flowStateRef.current = INITIAL_FLOW_CONTROL_STATE
        const unsubscribe = attachDataRef.current((bytes) => attachRef.current?.write(bytes))
        return unsubscribe
    }, [sessionId])

    return (
        <TerminalView
            fontSize={fontSize}
            theme={theme}
            onData={handleData}
            onResize={handleResize}
            onReady={handleReady}
            onWriteBacklogChange={handleWriteBacklogChange}
            attachRef={attachRef}
        />
    )
}
