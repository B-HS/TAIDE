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

export const recordImeDebug = (entry: Omit<ImeDebugEntry, 'at'>) => {
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
