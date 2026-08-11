const SHORT_HEX_NO_ALPHA_LENGTH = 3
const SHORT_HEX_WITH_ALPHA_LENGTH = 4

export const stripJsonComments = (source: string) => {
    let result = ''
    let inString = false
    let inLineComment = false
    let inBlockComment = false
    let escapeNext = false

    for (let index = 0; index < source.length; index += 1) {
        const char = source[index]
        const nextChar = source[index + 1]

        if (inLineComment) {
            if (char === '\n') {
                inLineComment = false
                result += char
            }
            continue
        }
        if (inBlockComment) {
            if (char === '*' && nextChar === '/') {
                inBlockComment = false
                index += 1
            }
            continue
        }
        if (inString) {
            result += char
            if (escapeNext) {
                escapeNext = false
            } else if (char === '\\') {
                escapeNext = true
            } else if (char === '"') {
                inString = false
            }
            continue
        }
        if (char === '"') {
            inString = true
            result += char
            continue
        }
        if (char === '/' && nextChar === '/') {
            inLineComment = true
            index += 1
            continue
        }
        if (char === '/' && nextChar === '*') {
            inBlockComment = true
            index += 1
            continue
        }
        result += char
    }

    return result.replace(/,(\s*[}\]])/g, '$1')
}

export const parseJsonc = (source: string): Record<string, unknown> => JSON.parse(stripJsonComments(source))

export const expandVscodeHex = (value: string) => {
    if (!value.startsWith('#')) return value
    const hex = value.slice(1)
    if (hex.length === SHORT_HEX_NO_ALPHA_LENGTH || hex.length === SHORT_HEX_WITH_ALPHA_LENGTH) {
        return `#${[...hex].map((digit) => `${digit}${digit}`).join('')}`
    }
    return value
}
