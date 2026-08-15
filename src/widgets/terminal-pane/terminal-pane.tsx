import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { ITheme } from '@xterm/xterm'
import { TerminalView } from '@features/terminal/terminal-view'
import type { TerminalAttachHandle, TerminalCursorStyle } from '@features/terminal/terminal-view'
import { INITIAL_FLOW_CONTROL_STATE, evaluateFlowControl, shouldTogglePause } from '@widgets/terminal-pane/terminal-flow-control'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'

export type TerminalPaneProps = {
    sessionId: string | null
    fontSize: number
    fontFamily: string
    theme: ITheme
    scrollback: number
    cursorStyle: TerminalCursorStyle
    cursorBlink: boolean
    commandSuccessColor: string | null
    commandFailureColor: string | null
    onWrite: (data: string) => void
    onResize: (cols: number, rows: number) => void
    onReady: (cols: number, rows: number) => void
    onSetPaused: (paused: boolean) => void
    attachData: (onData: (bytes: Uint8Array) => void) => () => void
}

export const TerminalPane: FC<TerminalPaneProps> = ({
    sessionId,
    fontSize,
    fontFamily,
    theme,
    scrollback,
    cursorStyle,
    cursorBlink,
    commandSuccessColor,
    commandFailureColor,
    onWrite,
    onResize,
    onReady,
    onSetPaused,
    attachData,
}) => {
    const attachRef = useRef<TerminalAttachHandle | null>(null)
    const flowStateRef = useRef(INITIAL_FLOW_CONTROL_STATE)
    const onWriteRef = useRef(onWrite)
    const onResizeRef = useRef(onResize)
    const onReadyRef = useRef(onReady)
    const onSetPausedRef = useRef(onSetPaused)
    const attachDataRef = useRef(attachData)

    const [isFocused, setIsFocused] = useState(false)

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

    const handleFocusChange = (focused: boolean) => setIsFocused(focused)

    useGlobalKeymap({
        'terminal-jump-to-previous-command': isFocused ? () => attachRef.current?.jumpToPreviousCommand() : undefined,
        'terminal-jump-to-next-command': isFocused ? () => attachRef.current?.jumpToNextCommand() : undefined,
    })

    useEffect(() => {
        if (!sessionId) return
        flowStateRef.current = INITIAL_FLOW_CONTROL_STATE
        const unsubscribe = attachDataRef.current((bytes) => attachRef.current?.write(bytes))
        return unsubscribe
    }, [sessionId])

    return (
        <TerminalView
            fontSize={fontSize}
            fontFamily={fontFamily}
            theme={theme}
            scrollback={scrollback}
            cursorStyle={cursorStyle}
            cursorBlink={cursorBlink}
            commandSuccessColor={commandSuccessColor}
            commandFailureColor={commandFailureColor}
            onData={handleData}
            onResize={handleResize}
            onReady={handleReady}
            onWriteBacklogChange={handleWriteBacklogChange}
            onFocusChange={handleFocusChange}
            attachRef={attachRef}
        />
    )
}
