import type { FC, RefObject } from 'react'
import { useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import type { ITheme } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { SearchAddon } from '@xterm/addon-search'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { WebLinksAddon } from '@xterm/addon-web-links'
import '@xterm/xterm/css/xterm.css'
import { resolveImeInput } from '@shared/lib/ime-input'

export type TerminalAttachHandle = {
    write: (data: Uint8Array) => void
}

export type TerminalCursorStyle = 'bar' | 'block' | 'underline'

export type TerminalViewProps = {
    fontSize: number
    fontFamily: string
    theme: ITheme
    scrollback: number
    cursorStyle: TerminalCursorStyle
    cursorBlink: boolean
    onData: (data: string) => void
    onResize: (cols: number, rows: number) => void
    onReady: (cols: number, rows: number) => void
    onWriteBacklogChange: (pendingBytes: number) => void
    attachRef: RefObject<TerminalAttachHandle | null>
}

export const TerminalView: FC<TerminalViewProps> = ({
    fontSize,
    fontFamily,
    theme,
    scrollback,
    cursorStyle,
    cursorBlink,
    onData,
    onResize,
    onReady,
    onWriteBacklogChange,
    attachRef,
}) => {
    const containerRef = useRef<HTMLDivElement>(null)
    const termRef = useRef<Terminal | null>(null)
    const fitRef = useRef<FitAddon | null>(null)
    const onDataRef = useRef(onData)
    const onResizeRef = useRef(onResize)
    const onReadyRef = useRef(onReady)
    const onWriteBacklogChangeRef = useRef(onWriteBacklogChange)
    const attachRefRef = useRef(attachRef)
    const initialFontSizeRef = useRef(fontSize)
    const initialFontFamilyRef = useRef(fontFamily)
    const initialThemeRef = useRef(theme)
    const initialScrollbackRef = useRef(scrollback)
    const initialCursorStyleRef = useRef(cursorStyle)
    const initialCursorBlinkRef = useRef(cursorBlink)

    useEffect(() => {
        onDataRef.current = onData
        onResizeRef.current = onResize
        onReadyRef.current = onReady
        onWriteBacklogChangeRef.current = onWriteBacklogChange
        attachRefRef.current = attachRef
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
            minimumContrastRatio: 1,
            drawBoldTextInBrightColors: true,
            smoothScrollDuration: 0,
        })

        const fit = new FitAddon()
        const search = new SearchAddon({ highlightLimit: 1000 })
        const unicode11 = new Unicode11Addon()
        const webLinks = new WebLinksAddon()

        term.loadAddon(fit)
        term.loadAddon(search)
        term.loadAddon(unicode11)
        term.loadAddon(webLinks)

        term.open(container)
        term.unicode.activeVersion = '11'

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
        }
        const handleImeInput = (event: Event) => {
            const input = event as InputEvent
            const resolved = resolveImeInput(input.inputType, input.data ?? '', composing, pendingReplaceLength)
            pendingReplaceLength = null
            if (!resolved) return
            composing = resolved.composing
            if (resolved.output) onDataRef.current(resolved.output)
        }
        textarea?.addEventListener('beforeinput', handleBeforeInput, true)
        textarea?.addEventListener('input', handleImeInput, true)

        const dataSubscription = term.onData((data) => onDataRef.current(data))
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
        }

        return () => {
            textarea?.removeEventListener('beforeinput', handleBeforeInput, true)
            textarea?.removeEventListener('input', handleImeInput, true)
            attachRefRef.current.current = null
            cancelAnimationFrame(resizeRafId)
            resizeObserver.disconnect()
            dataSubscription.dispose()
            resizeSubscription.dispose()
            webgl?.dispose()
            search.dispose()
            unicode11.dispose()
            webLinks.dispose()
            fit.dispose()
            term.dispose()
            termRef.current = null
            fitRef.current = null
        }
    }, [])

    return <div ref={containerRef} className='h-full w-full' />
}
