const CONTROL_END = 0x20
const BMP_END = 0xffff
const SURROGATE_START = 0xd800
const SURROGATE_END = 0xdfff

export const rustLiteral = (value) => {
    const encoded = Array.from(value)
        .map((character) => {
            const code = character.codePointAt(0)
            if (code == null || (code >= SURROGATE_START && code <= SURROGATE_END)) throw new Error('Invalid Unicode scalar in Emmet fixture')
            if (character === '\\') return '\\\\'
            if (character === '"') return '\\"'
            if (character === '\n') return '\\n'
            if (character === '\r') return '\\r'
            if (character === '\t') return '\\t'
            if (code < CONTROL_END || code > BMP_END) return `\\u{${code.toString(16)}}`
            return character
        })
        .join('')
    return `"${encoded}"`
}
