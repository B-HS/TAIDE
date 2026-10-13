import { parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { JSONDocumentSymbols } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonDocumentSymbols.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const URIS = [
    'file:///oracle.json',
    'file:///USER/keybindings.json',
    'vscode://defaultsettings/keybindings.json',
    'file:///user/keybindings.json?x',
    'file:///USER/KEYBINDINGS.JSON',
    'vscode://defaultsettings/KEYBINDINGS.JSON',
]
const RESULT_LIMITS = [null, 0, 1, 2, 4, 8]
const inputs = new Set([
    '',
    'null',
    'true',
    '1',
    '{}',
    '[]',
    '{',
    '[',
    '{"a"}',
    '{"a":}',
    '{"a":1,"b":true,"c":null,"d":"value","e":[],"f":{}}',
    '{"a":{"b":1,"c":[1,{"d":2}]},"e":[{"f":3},4]}',
    '[1,true,null,"value",[],{},[1,2],{"a":{"b":0}}]',
    '{"":"", " ":[], "\\n":{}, "\\r":null, "\\t":true, "\\ufeff":0, "\\u0085":false}',
    '{"𐐀":"漢字", "\\uD800":"\\uDFFF", "a\\nb":"line\\nvalue"}',
    '{"__proto__":0,"toString":1,"constructor":2,"//":3,"//":4,"a":5,"a":6}',
    '{"a"\n"b":2,"c":{"d"}}',
    '{"a":1,/*',
    '[{"a":0},/*',
    '[{"key":"ctrl+a","command":"test"},{"key":"ctrl+b"},{"missing":1}]',
    '[{"key":""},{"key":0},{"key":false},{"key":null},{"key":true},{"key":1},{"key":{}},{"key":[]},{"key":["a",null,["b",{}]]}]',
    '[{"key":"a","key":"b"},null,[{"key":"nested"}],{"key":"𐐀\\uD800"}]',
])
const NUMBERS = [
    '-0',
    '1e',
    '1.0',
    '1e2',
    '1e-6',
    '1e-7',
    '1e20',
    '1e21',
    '1e22',
    '1e309',
    '-1e309',
    '1e-400',
    '9007199254740993',
    '1.7976931348623157e308',
    '2.2250738585072014e-308',
    '5e-324',
    '2e-324',
    '-2e-324',
    '0.10000000000000001',
    '1000000000000000128',
]
for (const number of NUMBERS) {
    inputs.add(`{"a":${number}}`)
    inputs.add(`[{"key":${number}}]`)
}
for (const value of ['[]', '{}', '[1,2]', '{"x":0}', '"𐐀"', 'true', 'false', 'null']) {
    inputs.add(`{"a":${value},"b":${value},"c":${value}}`)
    inputs.add(`[{"a":${value}},${value},[${value}]]`)
}
const provider = new JSONDocumentSymbols(undefined)
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const safeName = (value) => {
    if (Array.isArray(value)) return value.map((child) => (child == null ? '' : safeName(child))).join(',')
    if (value != null && typeof value === 'object') return '[object Object]'
    return String(value)
}
const canonical = (symbols, document, hierarchical, corrected, exceeded) => {
    const pending = symbols.map((symbol) => ({ symbol, parent: -1 }))
    const parts = []
    for (let cursor = 0; cursor < pending.length; cursor += 1) {
        const { symbol, parent } = pending[cursor]
        const range = hierarchical ? symbol.range : symbol.location.range
        const selection = hierarchical ? symbol.selectionRange : range
        const start = document.offsetAt(range.start)
        const end = document.offsetAt(range.end)
        const selectionStart = document.offsetAt(selection.start)
        const selectionEnd = document.offsetAt(selection.end)
        const nameType = corrected ? 'string' : typeof symbol.name
        const sourceName = typeof symbol.name === 'object' ? JSON.stringify(symbol.name) : String(symbol.name)
        const name = hexUnits(corrected ? safeName(symbol.name) : sourceName)
        const detail = symbol.detail == null ? '-' : hexUnits(symbol.detail)
        const container = symbol.containerName == null ? '-' : hexUnits(symbol.containerName)
        parts.push(`${symbol.kind}:${parent}:${start}..${end}:${selectionStart}..${selectionEnd}:${nameType}:${name}:${detail}:${container};`)
        for (const child of symbol.children ?? []) pending.push({ symbol: child, parent: cursor })
    }
    return `${parts.join('')}|${exceeded}`
}
let corrections = 0
const cases = Array.from(inputs).flatMap((input) =>
    URIS.flatMap((uri) =>
        [false, true].flatMap((hierarchical) =>
            RESULT_LIMITS.map((resultLimit) => {
                const document = TextDocument.create(uri, 'json', 1, input)
                const parsed = parse(document, { collectComments: false })
                let exceeded = false
                const context = {
                    onResultLimitExceeded: () => {
                        exceeded = true
                    },
                }
                if (resultLimit != null) context.resultLimit = resultLimit
                const symbols = hierarchical
                    ? provider.findDocumentSymbols2(document, parsed, context)
                    : provider.findDocumentSymbols(document, parsed, context)
                const raw = canonical(symbols, document, hierarchical, false, exceeded)
                const expected = canonical(symbols, document, hierarchical, true, exceeded)
                if (raw !== expected) corrections += 1
                const limit = resultLimit == null ? 'None' : `Some(${resultLimit})`
                return `    (${rustLiteral(input)}, ${rustLiteral(uri)}, ${hierarchical}, ${limit}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`
            }),
        ),
    ),
)
await Bun.write(
    new URL('./json-symbols-oracle.rs', import.meta.url),
    `const CASES: &[(&str, &str, bool, Option<usize>, &str, &str)] = &[\n${cases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ inputs: inputs.size, cases: cases.length, corrections })}\n`)
