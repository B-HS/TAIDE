import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { ITheme } from '@xterm/xterm'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import { TerminalView } from '@features/terminal/terminal-view'
import type { TerminalAttachHandle, TerminalCursorStyle } from '@features/terminal/terminal-view'
import { INITIAL_FLOW_CONTROL_STATE, evaluateFlowControl, shouldTogglePause } from '@widgets/terminal-pane/terminal-flow-control'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'

export type TerminalPaneProps = {
    sessionId: string | null
    /** Whether this terminal's pane is the focused one — see `TerminalViewProps.autoFocus`. */
    autoFocus: boolean
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
    onOpenLink: (uri: string) => void
    onOpenFileLink: (match: TerminalLinkMatch) => void
    attachData: (onData: (bytes: Uint8Array) => void) => () => void
}

export const TerminalPane: FC<TerminalPaneProps> = ({
    sessionId,
    autoFocus,
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
    onOpenLink,
    onOpenFileLink,
    attachData,
}) => {
    const attachRef = useRef<TerminalAttachHandle | null>(null)
    const flowStateRef = useRef(INITIAL_FLOW_CONTROL_STATE)
    const onWriteRef = useRef(onWrite)
    const onResizeRef = useRef(onResize)
    const onReadyRef = useRef(onReady)
    const onSetPausedRef = useRef(onSetPaused)
    const onOpenLinkRef = useRef(onOpenLink)
    const onOpenFileLinkRef = useRef(onOpenFileLink)
    const attachDataRef = useRef(attachData)

    const [isFocused, setIsFocused] = useState(false)

    useEffect(() => {
        onWriteRef.current = onWrite
        onResizeRef.current = onResize
        onReadyRef.current = onReady
        onSetPausedRef.current = onSetPaused
        onOpenLinkRef.current = onOpenLink
        onOpenFileLinkRef.current = onOpenFileLink
        attachDataRef.current = attachData
    })

    const handleData = (data: string) => onWriteRef.current(data)

    const handleResize = (cols: number, rows: number) => onResizeRef.current(cols, rows)

    const handleReady = (cols: number, rows: number) => onReadyRef.current(cols, rows)

    const handleOpenLink = (uri: string) => onOpenLinkRef.current(uri)

    const handleOpenFileLink = (match: TerminalLinkMatch) => onOpenFileLinkRef.current(match)

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

    /**
     * `pty_set_paused` gates the reader thread for the *whole session*, and nothing in the backend
     * clears it on detach — so a pause this view raised (its write backlog crossed `HIGH_WATER`
     * during a burst) outlives the view that raised it. Switching tabs mid-burst unmounts this pane
     * with the pty still paused, freezing the child process — and the next mount starts from
     * `INITIAL_FLOW_CONTROL_STATE`, which believes it is *not* paused and therefore never sends a
     * resume, so the session stays frozen for good (audit §4-B D5, confirmed against
     * `domain::terminal::commands::pty_detach`, which touches only the subscriber list).
     *
     * Both ends of that gap are closed here: the cleanup resumes a pty this view left paused, so a
     * backgrounded terminal keeps making progress, and the attach resynchronizes unconditionally,
     * which also covers pauses no cleanup ever ran for (a reloaded webview, a window closed
     * mid-burst). Resuming an already-running pty is a no-op, so the redundant call costs one IPC
     * per attach and nothing else.
     */
    useEffect(() => {
        if (!sessionId) return
        flowStateRef.current = INITIAL_FLOW_CONTROL_STATE
        onSetPausedRef.current(false)
        const unsubscribe = attachDataRef.current((bytes) => attachRef.current?.write(bytes))
        return () => {
            unsubscribe()
            if (flowStateRef.current.paused) onSetPausedRef.current(false)
            flowStateRef.current = INITIAL_FLOW_CONTROL_STATE
        }
    }, [sessionId])

    return (
        <TerminalView
            autoFocus={autoFocus}
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
            onOpenLink={handleOpenLink}
            onOpenFileLink={handleOpenFileLink}
            attachRef={attachRef}
        />
    )
}
