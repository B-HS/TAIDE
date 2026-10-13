import { createScanner } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/main.js'
import { getSelectionRanges } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSelectionRanges.js'
import { parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const LOW_SURROGATE_START = 0xdc00
const SURROGATE_END = 0xdfff
const EOF_KIND = 17
const STRING_INPUTS = ['""', '"text"', '"𐐀漢字"', '"\\uD800\\uDFFF"', '"line\\nnext"', '"\\q"', '"𐐀', '"text']
const VALUE_INPUTS = [...STRING_INPUTS, 'null', 'true', 'false', '-0', '1.0', '1e2', '1e', '{}', '[]', '{"a":1}', '[1,2]']
const inputs = new Set(['', ' ', '/*', '//x\n', '{', '[', '{,}', '[,]', '{"a"\n"b":2}', '{"a":}', '{"a":1,/*', '["𐐀'])
for (const value of VALUE_INPUTS) {
    for (const input of [value, `[${value},null]`, `{"a":${value},"b":2}`, `[{"key":${value}},${value}]`]) inputs.add(input)
    for (const trivia of [' ', '\r\n', '/*x*/', '//x\n']) inputs.add(`{"a":${value}${trivia},"b":[${value},0]}`)
}
const canonical = (selection, document, input, corrected) => {
    const ranges = []
    let current = selection
    while (current) {
        let start = document.offsetAt(current.range.start)
        let end = document.offsetAt(current.range.end)
        if (corrected && start > 0 && input.charCodeAt(start) >= LOW_SURROGATE_START && input.charCodeAt(start) <= SURROGATE_END) start -= 1
        if (corrected && end < input.length && input.charCodeAt(end) >= LOW_SURROGATE_START && input.charCodeAt(end) <= SURROGATE_END) end += 1
        ranges.push(`${start}..${end};`)
        current = current.parent
    }
    return ranges.join('')
}
let corrections = 0
const cases = Array.from(inputs).flatMap((input) => {
    const document = TextDocument.create('file:///selection.json', 'json', 1, input)
    const parsed = parse(document, { collectComments: false })
    const bounded = parse(document, { collectComments: false })
    const scanner = createScanner(input, false)
    while (scanner.scan() !== EOF_KIND) {}
    bounded.visit((node) => {
        if (node.offset + node.length > input.length) {
            if (node.offset + node.length !== input.length + 1 || scanner.getTokenOffset() !== input.length || scanner.getTokenLength() !== 1)
                throw new Error('Unexpected source AST overflow in selection')
            node.length = input.length - node.offset
        }
        return true
    })
    const positions = []
    let offset = 0
    let byte = 0
    for (const character of input) {
        positions.push({ offset, byte })
        offset += character.length
        byte += new TextEncoder().encode(character).length
    }
    positions.push({ offset, byte })
    return positions.map(({ offset, byte }) => {
        const [selection] = getSelectionRanges(document, [document.positionAt(offset)], parsed)
        const [boundedSelection] = getSelectionRanges(document, [document.positionAt(offset)], bounded)
        const original = canonical(selection, document, input, false)
        const expected = canonical(boundedSelection, document, input, true)
        if (original !== expected) corrections += 1
        return `    (${rustLiteral(input)}, ${byte}, ${rustLiteral(original)}, ${rustLiteral(expected)}),`
    })
})
await Bun.write(new URL('./json-selection-oracle.rs', import.meta.url), `const CASES: &[(&str, usize, &str, &str)] = &[\n${cases.join('\n')}\n];\n`)
process.stdout.write(`${JSON.stringify({ inputs: inputs.size, cases: cases.length, corrections })}\n`)
