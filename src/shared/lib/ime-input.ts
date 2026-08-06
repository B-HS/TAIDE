const BACKSPACE = '\x7f'

export const INSERT_TEXT = 'insertText'
export const INSERT_REPLACEMENT_TEXT = 'insertReplacementText'

export type ImeInputResolution = {
    output: string
    composing: string
}

export const resolveImeInput = (inputType: string, data: string, composing: string): ImeInputResolution | null => {
    if (inputType === INSERT_REPLACEMENT_TEXT) return { output: BACKSPACE.repeat(composing.length) + data, composing: data }
    if (inputType === INSERT_TEXT) return { output: '', composing: data }
    return { output: '', composing: '' }
}
