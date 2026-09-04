import type { FC, RefObject } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { ITheme } from '@xterm/xterm'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import { TerminalView } from '@features/terminal/terminal-view'
import type { TerminalAttachHandle, TerminalCursorStyle } from '@features/terminal/terminal-view'
import type { SplitEdge } from '@features/tab/tab-context-menu'
import { TerminalContextMenu } from '@features/terminal/terminal-context-menu'
import { INITIAL_FLOW_CONTROL_STATE, evaluateFlowControl, shouldTogglePause } from '@widgets/terminal-pane/terminal-flow-control'
import { probeTerminalClipboardAvailability } from '@widgets/terminal-pane/terminal-clipboard-availability'
import { resolveSplitAvailability } from '@widgets/terminal-pane/terminal-split-availability'
import { MIN_PANEL_SIZE_PX } from '@shared/constants/layout'
import { useGlobalKeymap } from '@shared/hooks/use-global-keymap'

type TerminalMenuSnapshot = {
    canCopy: boolean
    canPaste: boolean
    splitAvailability: Record<SplitEdge, boolean>
}

const CLOSED_MENU_SNAPSHOT: TerminalMenuSnapshot = {
    canCopy: false,
    canPaste: false,
    splitAvailability: { left: false, right: false, top: false, bottom: false },
}

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
    /** The pane's terminal area, handed down from `pane-node-view.tsx` — measured once per menu open to decide which splits fit. */
    paneElementRef: RefObject<HTMLDivElement | null>
    resizerThicknessPx: number
    onWrite: (data: string) => void
    onResize: (cols: number, rows: number) => void
    onReady: (cols: number, rows: number) => void
    onSetPaused: (paused: boolean) => void
    onOpenLink: (uri: string) => void
    onOpenFileLink: (match: TerminalLinkMatch) => void
    onSplitNewTerminal: (edge: SplitEdge) => void
    onNewTerminal: () => void
    onKillTerminal: () => void
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
    paneElementRef,
    resizerThicknessPx,
    onWrite,
    onResize,
    onReady,
    onSetPaused,
    onOpenLink,
    onOpenFileLink,
    onSplitNewTerminal,
    onNewTerminal,
    onKillTerminal,
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
    const [menuSnapshot, setMenuSnapshot] = useState(CLOSED_MENU_SNAPSHOT)

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

    /**
     * Everything the menu shows about the terminal's *current* condition is sampled here, once,
     * while it opens — the selection, whether this runtime even has a clipboard, and the pane's own
     * size. Sampling on open rather than subscribing keeps the terminal free of a `ResizeObserver`
     * and of a re-render per selection change; a menu that is already open cannot go stale anyway,
     * since it swallows the input that would change any of the three.
     */
    const handleMenuOpenChange = (open: boolean) => {
        if (!open) return
        const paneRect = paneElementRef.current?.getBoundingClientRect()
        const clipboard = probeTerminalClipboardAvailability()
        setMenuSnapshot({
            canCopy: clipboard.canCopy && (attachRef.current?.hasSelection() ?? false),
            canPaste: clipboard.canPaste,
            splitAvailability: resolveSplitAvailability({
                paneWidthPx: paneRect?.width ?? 0,
                paneHeightPx: paneRect?.height ?? 0,
                minPaneSizePx: MIN_PANEL_SIZE_PX,
                resizerThicknessPx,
            }),
        })
    }

    const handleCopySelection = () => {
        const selection = attachRef.current?.getSelection() ?? ''
        if (selection) void navigator.clipboard.writeText(selection).catch(() => undefined)
    }

    const handlePasteClipboard = () => {
        void navigator.clipboard
            .readText()
            .then((text) => attachRef.current?.paste(text))
            .catch(() => undefined)
    }

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
        <TerminalContextMenu
            canCopy={menuSnapshot.canCopy}
            canPaste={menuSnapshot.canPaste}
            splitAvailability={menuSnapshot.splitAvailability}
            onOpenChange={handleMenuOpenChange}
            onRestoreFocus={() => attachRef.current?.focus()}
            onCopy={handleCopySelection}
            onPaste={handlePasteClipboard}
            onSelectAll={() => attachRef.current?.selectAll()}
            onClear={() => attachRef.current?.clear()}
            onSplit={onSplitNewTerminal}
            onNewTerminal={onNewTerminal}
            onKill={onKillTerminal}>
            <div className='h-full w-full'>
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
            </div>
        </TerminalContextMenu>
    )
}
