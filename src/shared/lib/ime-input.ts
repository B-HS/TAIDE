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
