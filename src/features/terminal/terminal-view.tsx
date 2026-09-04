import type { FC, RefObject } from 'react'
import { useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import type { ITheme } from '@xterm/xterm'
import type { TerminalCursorStyle } from '@shared/api/bindings'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { SearchAddon } from '@xterm/addon-search'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import { INSERT_TEXT, createInsertTextDeduper, resolveImeInput } from '@shared/lib/ime-input'
import { recordImeDebug } from '@shared/lib/ime-debug'
import { IS_MAC } from '@shared/constants/platform'
import type { TerminalLinkMatch } from '@shared/lib/terminal-link'
import type { CommandBlockDecorationColors } from '@features/terminal/terminal-osc133'
import { attachOsc133BlockTracker } from '@features/terminal/terminal-osc133'
import { createTerminalFileLinkProvider } from '@features/terminal/terminal-file-link'

const OVERVIEW_RULER_WIDTH_PX = 14
const SEARCH_HIGHLIGHT_LIMIT = 1000
const SHIFT_ENTER_LINE_FEED = '\n'

/**
 * xterm's built-in web-links handler activates on any click, which collides with terminal
 * text selection and cursor placement. TAIDE gates link activation to Cmd-click (mac) /
 * Ctrl-click (non-mac) / Alt-click, matching the modifier convention editors use for "open
 * reference". `isMac` defaults to {@link IS_MAC} and exists as a parameter purely so tests can
 * cover both platforms without touching `navigator` — production call sites never pass it.
 *
 * The same gate covers all three link kinds so they behave identically: plain-text URLs
 * (`WebLinksAddon`), `path:line:col` matches (`terminal-file-link.ts`), and OSC 8 hyperlinks.
 * The last of those needs the terminal's `linkHandler` option, because xterm registers its own
 * `OscLinkProvider` at priority 0 and would otherwise activate them through its default handler
 * — a `confirm()` dialog WKWebView never answers, followed by a `window.open()` the app must
 * never take (`docs/features/terminal.md` §6.1).
 */
export const shouldActivateTerminalLink = (event: Pick<MouseEvent, 'metaKey' | 'altKey' | 'ctrlKey'>, isMac: boolean = IS_MAC) =>
    event.altKey || (isMac ? event.metaKey : event.ctrlKey)

/**
 * xterm.js encodes Shift+Enter identically to Enter (a bare CR) because it speaks neither the
 * kitty keyboard protocol nor xterm's modifyOtherKeys, so TUIs that give Shift+Enter its own
 * meaning — Claude Code inserts a newline instead of submitting — can never see it. TAIDE
 * translates the combo to LF, the byte Ctrl+J produces, which Claude Code documents as the
 * universal "insert newline" key; plain shells bind CR and LF to the same accept-line, so the
 * mapping is behavior-preserving at a prompt. Only a plain shift-modified Enter keydown outside
 * IME composition qualifies — anything else stays on xterm's own keyboard pipeline. Decision:
 * `docs/acknowledge/2026-08-29-terminal-shift-enter-decision.md`.
 */
export const shouldTranslateShiftEnterToLineFeed = (
    event: Pick<KeyboardEvent, 'type' | 'key' | 'shiftKey' | 'altKey' | 'ctrlKey' | 'metaKey' | 'isComposing'>,
) => event.type === 'keydown' && event.key === 'Enter' && event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey && !event.isComposing

export type TerminalAttachHandle = {
    write: (data: Uint8Array) => void
    jumpToPreviousCommand: () => void
    jumpToNextCommand: () => void
}

export type { TerminalCursorStyle }

export type TerminalViewProps = {
    /**
     * Moves keyboard focus into the terminal as soon as it is created. Activating a terminal tab
     * mounts this view (`pane-node-view.tsx` renders only each pane's active tab), and without this
     * the freshly shown terminal had no focus at all — keystrokes went to whatever was focused
     * before, and the user had to click the black rectangle first (audit §4-B C14). Matches what
     * `code-editor.tsx` already does when it attaches a model. Passed as `false` for a pane that is
     * not the focused one, so restoring a session with terminals in several panes doesn't have them
     * race the focused pane's editor for focus.
     */
    autoFocus: boolean
    fontSize: number
    fontFamily: string
    theme: ITheme
    scrollback: number
    cursorStyle: TerminalCursorStyle
    cursorBlink: boolean
    commandSuccessColor: string | null
    commandFailureColor: string | null
    onData: (data: string) => void
    onResize: (cols: number, rows: number) => void
    onReady: (cols: number, rows: number) => void
    onWriteBacklogChange: (pendingBytes: number) => void
    onFocusChange: (isFocused: boolean) => void
    onOpenLink: (uri: string) => void
    onOpenFileLink: (match: TerminalLinkMatch) => void
    attachRef: RefObject<TerminalAttachHandle | null>
}

export const TerminalView: FC<TerminalViewProps> = ({
    autoFocus,
    fontSize,
    fontFamily,
    theme,
    scrollback,
    cursorStyle,
    cursorBlink,
    commandSuccessColor,
    commandFailureColor,
    onData,
    onResize,
    onReady,
    onWriteBacklogChange,
    onFocusChange,
    onOpenLink,
    onOpenFileLink,
    attachRef,
}) => {
    const containerRef = useRef<HTMLDivElement>(null)
    const termRef = useRef<Terminal | null>(null)
    const fitRef = useRef<FitAddon | null>(null)
    const onDataRef = useRef(onData)
    const onResizeRef = useRef(onResize)
    const onReadyRef = useRef(onReady)
    const onWriteBacklogChangeRef = useRef(onWriteBacklogChange)
    const onFocusChangeRef = useRef(onFocusChange)
    const onOpenLinkRef = useRef(onOpenLink)
    const onOpenFileLinkRef = useRef(onOpenFileLink)
    const attachRefRef = useRef(attachRef)
    const initialFontSizeRef = useRef(fontSize)
    const initialFontFamilyRef = useRef(fontFamily)
    const initialThemeRef = useRef(theme)
    const initialScrollbackRef = useRef(scrollback)
    const initialCursorStyleRef = useRef(cursorStyle)
    const initialCursorBlinkRef = useRef(cursorBlink)
    const commandBlockColorsRef = useRef<CommandBlockDecorationColors>({ success: commandSuccessColor, failure: commandFailureColor })
    const initialAutoFocusRef = useRef(autoFocus)

    useEffect(() => {
        onDataRef.current = onData
        onResizeRef.current = onResize
        onReadyRef.current = onReady
        onWriteBacklogChangeRef.current = onWriteBacklogChange
        onFocusChangeRef.current = onFocusChange
        onOpenLinkRef.current = onOpenLink
        onOpenFileLinkRef.current = onOpenFileLink
        attachRefRef.current = attachRef
        commandBlockColorsRef.current = { success: commandSuccessColor, failure: commandFailureColor }
    })

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.fontSize = fontSize
        fitRef.current?.fit()
    }, [fontSize])

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.fontFamily = fontFamily
        fitRef.current?.fit()
    }, [fontFamily])

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.theme = theme
    }, [theme])

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.scrollback = scrollback
    }, [scrollback])

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.cursorStyle = cursorStyle
    }, [cursorStyle])

    useEffect(() => {
        const term = termRef.current
        if (!term) return
        term.options.cursorBlink = cursorBlink
    }, [cursorBlink])

    useEffect(() => {
        const container = containerRef.current
        if (!container) return

        const term = new Terminal({
            allowProposedApi: true,
            fontSize: initialFontSizeRef.current,
            fontFamily: initialFontFamilyRef.current,
            theme: initialThemeRef.current,
            scrollback: initialScrollbackRef.current,
            cursorBlink: initialCursorBlinkRef.current,
            cursorStyle: initialCursorStyleRef.current,
            macOptionIsMeta: true,
            altClickMovesCursor: false,
            minimumContrastRatio: 1,
            drawBoldTextInBrightColors: true,
            smoothScrollDuration: 0,
            overviewRuler: { width: OVERVIEW_RULER_WIDTH_PX },
            linkHandler: {
                activate: (event, text) => {
                    if (!shouldActivateTerminalLink(event)) return
                    onOpenLinkRef.current(text)
                },
                allowNonHttpProtocols: false,
            },
        })

        const fit = new FitAddon()
        const search = new SearchAddon({ highlightLimit: SEARCH_HIGHLIGHT_LIMIT })
        const unicode11 = new Unicode11Addon()
        const webLinks = new WebLinksAddon((event, uri) => {
            if (!shouldActivateTerminalLink(event)) return
            onOpenLinkRef.current(uri)
        })
        const fileLinkProvider = createTerminalFileLinkProvider(term, (match, event) => {
            if (!shouldActivateTerminalLink(event)) return
            onOpenFileLinkRef.current(match)
        })

        term.loadAddon(fit)
        term.loadAddon(search)
        term.loadAddon(unicode11)
        term.loadAddon(webLinks)
        const fileLinkDisposable = term.registerLinkProvider(fileLinkProvider)

        term.open(container)
        term.unicode.activeVersion = '11'
        term.attachCustomKeyEventHandler((event) => {
            if (!shouldTranslateShiftEnterToLineFeed(event)) return true
            event.preventDefault()
            onDataRef.current(SHIFT_ENTER_LINE_FEED)
            return false
        })

        let webgl: WebglAddon | null = null
        const loadWebgl = () => {
            const addon = new WebglAddon()
            addon.onContextLoss(() => {
                addon.dispose()
                loadWebgl()
            })
            term.loadAddon(addon)
            webgl = addon
        }
        loadWebgl()

        fit.fit()
        if (initialAutoFocusRef.current) term.focus()

        const pendingRef = { current: 0 }
        const reportBacklog = () => onWriteBacklogChangeRef.current(pendingRef.current)

        let resizeRafId = 0
        const resizeObserver = new ResizeObserver(() => {
            cancelAnimationFrame(resizeRafId)
            resizeRafId = requestAnimationFrame(() => {
                const dimensions = fit.proposeDimensions()
                if (!dimensions || !Number.isFinite(dimensions.cols) || !Number.isFinite(dimensions.rows)) return
                if (dimensions.cols !== term.cols || dimensions.rows !== term.rows) fit.fit()
            })
        })
        resizeObserver.observe(container)

        const textarea = term.textarea
        let composing = ''
        let pendingReplaceLength: number | null = null
        const handleBeforeInput = (event: Event) => {
            const input = event as InputEvent
            if (typeof input.getTargetRanges !== 'function') return
            const range = input.getTargetRanges()[0]
            pendingReplaceLength = range ? range.endOffset - range.startOffset : null
            recordImeDebug({
                source: 'beforeinput',
                inputType: input.inputType,
                data: input.data ?? '',
                rangeLength: pendingReplaceLength,
                composing,
                output: '',
            })
        }
        const insertTextDeduper = createInsertTextDeduper()
        const handleImeInput = (event: Event) => {
            const input = event as InputEvent
            const data = input.data ?? ''
            const resolved = resolveImeInput(input.inputType, data, composing, pendingReplaceLength)
            const insertTextVerdict = input.inputType === INSERT_TEXT && data ? insertTextDeduper.onInsertText(data, performance.now()) : null
            recordImeDebug({
                source: 'input',
                inputType: `${input.inputType}${insertTextVerdict ? `:${insertTextVerdict}` : ''}`,
                data,
                rangeLength: pendingReplaceLength,
                composing: resolved?.composing ?? '',
                output: insertTextVerdict === 'self-send' ? data : (resolved?.output ?? ''),
            })
            pendingReplaceLength = null
            if (insertTextVerdict === 'self-send') onDataRef.current(data)
            if (!resolved) return
            composing = resolved.composing
            if (resolved.output) onDataRef.current(resolved.output)
        }
        textarea?.addEventListener('beforeinput', handleBeforeInput, true)
        textarea?.addEventListener('input', handleImeInput, true)

        const handleFocus = () => onFocusChangeRef.current(true)
        const handleBlur = () => onFocusChangeRef.current(false)
        textarea?.addEventListener('focus', handleFocus)
        textarea?.addEventListener('blur', handleBlur)

        const osc133Tracker = attachOsc133BlockTracker(term, commandBlockColorsRef)

        const dataSubscription = term.onData((data) => {
            const verdict = insertTextDeduper.onXtermData(data, performance.now())
            recordImeDebug({ source: 'data', inputType: verdict, data, rangeLength: null, composing, output: verdict === 'forward' ? data : '' })
            if (verdict === 'drop') return
            onDataRef.current(data)
        })
        const resizeSubscription = term.onResize(({ cols, rows }) => onResizeRef.current(cols, rows))

        termRef.current = term
        fitRef.current = fit
        onReadyRef.current(term.cols, term.rows)
        attachRefRef.current.current = {
            write: (data) => {
                pendingRef.current += data.byteLength
                reportBacklog()
                term.write(data, () => {
                    pendingRef.current = Math.max(0, pendingRef.current - data.byteLength)
                    reportBacklog()
                })
            },
            jumpToPreviousCommand: () => osc133Tracker.jumpToPreviousCommand(),
            jumpToNextCommand: () => osc133Tracker.jumpToNextCommand(),
        }

        return () => {
            textarea?.removeEventListener('beforeinput', handleBeforeInput, true)
            textarea?.removeEventListener('input', handleImeInput, true)
            textarea?.removeEventListener('focus', handleFocus)
            textarea?.removeEventListener('blur', handleBlur)
            attachRefRef.current.current = null
            cancelAnimationFrame(resizeRafId)
            resizeObserver.disconnect()
            dataSubscription.dispose()
            resizeSubscription.dispose()
            osc133Tracker.dispose()
            webgl?.dispose()
            search.dispose()
            unicode11.dispose()
            webLinks.dispose()
            fileLinkDisposable.dispose()
            fit.dispose()
            term.dispose()
            termRef.current = null
            fitRef.current = null
        }
    }, [])

    return <div ref={containerRef} className='h-full w-full' />
}
