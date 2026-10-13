import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'
import { createScanner } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/main.js'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const EOF = 17
const LINE_COMMENT = 12
const BLOCK_COMMENT = 13
const LAST_TRIVIA = 15
const END_COMMENT = 1
const CONTROL_COUNT = 32
const alphabet = ['{', '}', '[', ']', ':', ',', '"', '\\', '/', '*', '-', '.', 'e', '+', '0', '1', 't', ' ', '\r', '\n', '\t']
const corpus = [
    '',
    '{}',
    '[]',
    '{"name": "value", "number": -1.25e+3, "items": [true, false, null]}',
    '{\r\n\t"a":1,// note\n"b":"x"\n}',
    '"\\uD800"',
    '"\\uDC00"',
    '"\\uD800\\uDC00"',
    '"\\u0041\\u0000"',
    '"\\u"',
    '"\\u12"',
    '"\\u12xz"',
    '"\\a"',
    '"\\\\\\\"\\/\\b\\f\\n\\r\\t"',
    '"漢\u{10400}한"',
    '\u{10400} // note\r\nnull',
    '/*a\r\nb\rc\nd*/ true',
    '/*x\n',
    '/*x\r',
    '/*x\r\n',
    '"a\\\nb"',
    '"a\u2028b\u2029c"',
    '\u00a0true\ufeff',
    '1e999',
    '-0',
    '-0.0',
    '00',
    '012.3',
    '1.',
    '-1e+',
    '-1e-2',
    'falsey true- null0',
    '/comment',
]
const inputs = new Set(corpus)
for (let code = 0; code < CONTROL_COUNT; code += 1) {
    inputs.add(String.fromCharCode(code))
    inputs.add(`"a${String.fromCharCode(code)}b"`)
}
for (const first of alphabet) {
    inputs.add(first)
    for (const second of alphabet) {
        inputs.add(first + second)
        for (const third of alphabet) inputs.add(first + second + third)
    }
}
const units = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const describe = (tokens) =>
    tokens.map((token) => `${token.kind}:${token.start}..${token.end}:${token.line}:${token.column}:${token.error}:${units(token.value)};`).join('')
const scan = (input, ignoreTrivia) => {
    const scanner = createScanner(input, ignoreTrivia)
    const tokens = []
    while (true) {
        const kind = scanner.scan()
        tokens.push({
            kind,
            start: scanner.getTokenOffset(),
            end: scanner.getTokenOffset() + scanner.getTokenLength(),
            line: scanner.getTokenStartLine(),
            column: scanner.getTokenStartCharacter(),
            error: scanner.getTokenError(),
            value: scanner.getTokenValue(),
        })
        if (kind === EOF) return tokens
    }
}
const correct = (input, source) => {
    const document = TextDocument.create('file:///synthetic/json-scanner', 'json', 0, input)
    return source.map((token) => {
        let value = token.value
        let end = token.end
        let line = token.line
        let column = token.column
        if (token.kind === LINE_COMMENT || token.kind === BLOCK_COMMENT) {
            const original = input.slice(Math.max(0, token.start - 1), Math.min(token.end, input.length))
            if (value !== original) throw new Error('Unexpected source comment value')
            value = input.slice(token.start, Math.min(end, input.length))
        }
        if (end > input.length) {
            if (end !== input.length + 1 || !(input.endsWith('/*') || input.endsWith('\r\n')) || ![BLOCK_COMMENT, EOF].includes(token.kind))
                throw new Error(`Unexpected source range overflow: ${JSON.stringify({ input, token })}`)
            end = input.length
        }
        const position = document.positionAt(token.start)
        const actualLine = position.line
        const actualColumn = position.character
        if (line !== actualLine || column !== actualColumn) {
            const incompleteComment = source.some((item) => item.kind === BLOCK_COMMENT && item.error === END_COMMENT)
            if (!incompleteComment && !input.includes('\\\n') && !input.includes('\\\r')) throw new Error('Unexpected source physical position')
            line = actualLine
            column = actualColumn
        }
        return { ...token, end, line, column, value }
    })
}
const cases = []
let changed = 0
for (const input of inputs) {
    const all = scan(input, false)
    const corrected = correct(input, all)
    for (const ignoreTrivia of [false, true]) {
        const source = scan(input, ignoreTrivia)
        const expected = corrected.filter((token) => !ignoreTrivia || token.kind < LINE_COMMENT || token.kind > LAST_TRIVIA)
        const raw = describe(source)
        const fixed = describe(expected)
        if (raw !== fixed) changed += 1
        cases.push(`    (${rustLiteral(input)}, ${ignoreTrivia}, ${rustLiteral(raw)}, ${rustLiteral(fixed)}),`)
    }
}
const output = new URL('./json-scanner-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, bool, &str, &str)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('JSON scanner fixture formatting failed')
console.info(JSON.stringify({ inputs: inputs.size, cases: cases.length, correctedCases: changed, source: 'monaco-editor/esm/external/jsonc-parser' }))
