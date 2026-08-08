const BACKSPACE = '\x7f'

export const INSERT_TEXT = 'insertText'
export const INSERT_REPLACEMENT_TEXT = 'insertReplacementText'

export type ImeInputResolution = {
    output: string
    composing: string
}

export const resolveImeInput = (
    inputType: string,
    data: string,
    composing: string,
    replaceLength: number | null = null,
): ImeInputResolution | null => {
    if (inputType === INSERT_REPLACEMENT_TEXT) {
        const eraseCount = replaceLength ?? composing.length
        const retained = composing.slice(0, Math.max(0, composing.length - eraseCount))
        return { output: BACKSPACE.repeat(eraseCount) + data, composing: retained + data }
    }
    if (inputType === INSERT_TEXT) return { output: '', composing: data }
    return { output: '', composing: '' }
}

export const IME_DUPLICATE_WINDOW_MS = 50

/**
 * Deduplicates the insertText send path between xterm and the IME adapter.
 * WKWebView intermittently makes xterm skip sending insertText data, so the
 * adapter self-sends when no matching xterm data was observed, and suppresses
 * a late duplicate from xterm exactly once.
 */
export const createInsertTextDeduper = () => {
    let lastData: string | null = null
    let lastDataAt = 0
    let suppress: string | null = null
    let suppressAt = 0

    const onXtermData = (data: string, at: number) => {
        if (suppress === data && at - suppressAt <= IME_DUPLICATE_WINDOW_MS) {
            suppress = null
            return 'drop' as const
        }
        suppress = null
        lastData = data
        lastDataAt = at
        return 'forward' as const
    }

    const onInsertText = (data: string, at: number) => {
        if (lastData === data && at - lastDataAt <= IME_DUPLICATE_WINDOW_MS) {
            lastData = null
            return 'already-sent' as const
        }
        suppress = data
        suppressAt = at
        return 'self-send' as const
    }

    return { onXtermData, onInsertText }
}
