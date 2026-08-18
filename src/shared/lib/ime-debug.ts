export type ImeDebugSource = 'beforeinput' | 'input' | 'data'

export type ImeDebugEntry = {
    at: number
    source: ImeDebugSource
    inputType: string
    data: string
    rangeLength: number | null
    composing: string
    output: string
}

const MAX_IME_DEBUG_ENTRIES = 300
const MAX_RECORDED_TEXT_LENGTH = 40

const entries: ImeDebugEntry[] = []

let imeDebugEnabled = false

/**
 * Collection defaults to off: every keystroke's raw text used to be recorded unconditionally,
 * holding user-typed terminal input in memory and exposing it verbatim through the
 * clipboard-copy command. Callers opt in explicitly when they need the trace for debugging.
 */
export const setImeDebugEnabled = (enabled: boolean) => {
    imeDebugEnabled = enabled
    if (!enabled) entries.splice(0)
}

export const isImeDebugEnabled = () => imeDebugEnabled

export const recordImeDebug = (entry: Omit<ImeDebugEntry, 'at'>) => {
    if (!imeDebugEnabled) return
    entries.push({
        ...entry,
        at: performance.now(),
        data: entry.data.slice(0, MAX_RECORDED_TEXT_LENGTH),
        output: entry.output.slice(0, MAX_RECORDED_TEXT_LENGTH),
    })
    if (entries.length > MAX_IME_DEBUG_ENTRIES) entries.splice(0, entries.length - MAX_IME_DEBUG_ENTRIES)
}

export const buildImeDebugReport = () =>
    entries
        .map(
            (entry) =>
                `${entry.at.toFixed(1)}ms ${entry.source} type=${entry.inputType || '-'} data=${JSON.stringify(entry.data)} range=${entry.rangeLength ?? '-'} composing=${JSON.stringify(entry.composing)} out=${JSON.stringify(entry.output)}`,
        )
        .join('\n')

export const clearImeDebug = () => {
    entries.splice(0)
}
