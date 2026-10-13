import { createScanner } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/main.js'
import { parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const COMBINATION_DEPTH = 3
const KINDS = ['object', 'array', 'property', 'string', 'number', 'boolean', 'null']
const FLOAT_BYTES = 8
const FLOAT_HEX_LENGTH = 16
const LONG_NUMBER_DIGITS = 4096
const EOF_KIND = 17
const LOW_SURROGATE_START = 0xdc00
const SURROGATE_END = 0xdfff
const INPUT_ALPHABET = ['{', '}', '[', ']', ':', ',', '"', 'a', '0', ' ', '\n', '/']
const floatBytes = new DataView(new ArrayBuffer(FLOAT_BYTES))
const inputs = new Set([''])
let prefixes = ['']
for (let depth = 1; depth <= COMBINATION_DEPTH; depth += 1) {
    prefixes = prefixes.flatMap((prefix) => INPUT_ALPHABET.map((character) => prefix + character))
    for (const input of prefixes) inputs.add(input)
}
const structured = [
    '{"a":1,"b":[true,false,null,{"c":"value"}]}',
    '{"a":1,"a":2,"a":3,"//":1,"//":2}',
    '{"__proto__":1,"constructor":2,"toString":3,"__proto__":4}',
    '{"a"\n"b":2,"c" 3,unquoted:true}',
    '{"a":,"b":2,"c":[],"d":{"e"}}',
    '[1,,2,3 4,true,{},[null,]]',
    '{"a":1,/* comment */"b":2}// tail',
    '{"𐐀":"한글\\uD800\\uDC00\\uDFFF","漢字":[1e2,-0,1.0]}',
    '{"a":"\\q","b":"\\u0","c":"\\\n"}',
    '{"a":1e,"b":1.,"c":1e309,"d":-1e-400}',
    '{"a":1} {"b":2}',
    '[[[[{"a":[1,2]}]]]]',
    '{"a":1,/*',
    '{"a":1,/*x\r\n',
    '["𐐀',
    '{"𐐀',
    '/*',
    '/*x\r\n',
    'true',
    'false',
    'null',
    '1e309',
    '-0',
    '1e-400',
    '1.0',
    '1e2',
]
for (const input of structured) {
    inputs.add(input)
    const characters = Array.from(input)
    for (let length = 1; length < characters.length; length += 1) inputs.add(characters.slice(0, length).join(''))
    for (let index = 0; index < characters.length; index += 1) {
        inputs.add(characters.toSpliced(index, 1).join(''))
        inputs.add(characters.toSpliced(index, 0, ',').join(''))
    }
}
const TRIVIA = ['\r\n', '\t', '\u00a0', '\ufeff', '\u2028', '\u2029', '\u0085', '/*x*/', '//x\n']
for (const trivia of TRIVIA) {
    for (const input of ['{}', '[1,2]', '{"a":true}', '"𐐀', '{"𐐀']) {
        inputs.add(`${trivia}${input}${trivia}`)
    }
}
for (const number of [
    '9007199254740991',
    '9007199254740992',
    '9007199254740993',
    '1.7976931348623157e308',
    '1.7976931348623159e308',
    '2.2250738585072014e-308',
    '5e-324',
    '4e-324',
    '2e-324',
    '-2e-324',
    '0.10000000000000001',
    '1e' + '9'.repeat(LONG_NUMBER_DIGITS),
    '-1e-' + '9'.repeat(LONG_NUMBER_DIGITS),
    '9'.repeat(LONG_NUMBER_DIGITS),
    '0.' + '0'.repeat(LONG_NUMBER_DIGITS) + '1',
])
    inputs.add(number)
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const numberBits = (value) => {
    floatBytes.setFloat64(0, value)
    return floatBytes.getBigUint64(0).toString(16).padStart(FLOAT_HEX_LENGTH, '0')
}
const canonical = (json, document, corrected) => {
    const text = document.getText()
    const scanner = createScanner(text, false)
    while (scanner.scan() !== EOF_KIND) {}
    const overflow = scanner.getTokenOffset() + scanner.getTokenLength() - text.length
    if (overflow !== 0 && overflow !== 1) throw new Error('Unexpected source scanner EOF overflow')
    if (overflow === 1 && (scanner.getTokenOffset() !== text.length || scanner.getTokenLength() !== 1))
        throw new Error('Unexpected source scanner EOF overflow layout')
    const chunks = []
    const visit = (node, parent) => {
        const index = chunks.length
        const start = node.offset
        if (start + node.length > text.length && start + node.length !== text.length + overflow) throw new Error('Unexpected source AST overflow')
        const end = corrected ? Math.min(text.length, start + node.length) : start + node.length
        if (end < start) throw new Error('Source AST range starts outside input')
        let value = ''
        if (node.type === 'string') value = hexUnits(node.value)
        if (node.type === 'number') value = Number.isNaN(node.value) ? 'nan' : numberBits(node.value)
        if (node.type === 'boolean') value = String(node.value)
        const colon = node.type === 'property' ? node.colonOffset : -1
        const integer = node.type === 'number' ? Number(node.isInteger) : 1
        chunks.push(`${KINDS.indexOf(node.type)}:${start}..${end}:${parent}:${colon}:${integer}:${value};`)
        for (const child of node.children) visit(child, index)
    }
    if (json.root) visit(json.root, -1)
    let previousStart = -1
    const errors = json.syntaxErrors.map((diagnostic) => {
        let start = document.offsetAt(diagnostic.range.start)
        let end = document.offsetAt(diagnostic.range.end)
        const sourceStart = start
        if (corrected && overflow === 1 && start === text.length && end === start) {
            start -= 1
            while (start > 0 && /\s/.test(text.charAt(start))) start -= 1
            end = start + 1
        }
        if (corrected && start > 0 && text.charCodeAt(start) >= LOW_SURROGATE_START && text.charCodeAt(start) <= SURROGATE_END) start -= 1
        if (corrected && end < text.length && text.charCodeAt(end) >= LOW_SURROGATE_START && text.charCodeAt(end) <= SURROGATE_END) end += 1
        if (corrected && start === previousStart && start !== sourceStart) return ''
        previousStart = start
        return `${diagnostic.code}:${diagnostic.severity}:${start}..${end};`
    })
    const comments = (json.comments ?? []).map((range) => `${document.offsetAt(range.start)}..${document.offsetAt(range.end)};`)
    return `${chunks.join('')}|${errors.join('')}|${comments.join('')}`
}
let corrections = 0
const cases = Array.from(inputs).flatMap((input) =>
    [false, true].map((collectComments) => {
        const document = TextDocument.create('file:///oracle.json', 'json', 1, input)
        const parsed = parse(document, { collectComments })
        const raw = canonical(parsed, document, false)
        const expected = canonical(parsed, document, true)
        if (raw !== expected) corrections += 1
        return `    (${rustLiteral(input)}, ${collectComments}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`
    }),
)
const queryInputs = [...structured, '[𐐀', '{𐐀', '{"a"\n"b":0}', '', '{}', '[,]', '{,}']
const queries = queryInputs.flatMap((input) => {
    const document = TextDocument.create('file:///oracle.json', 'json', 1, input)
    const parsed = parse(document, { collectComments: true })
    const indices = new Map()
    parsed.visit((node) => {
        indices.set(node, indices.size)
        if (node.offset + node.length > input.length) {
            if (node.offset + node.length !== input.length + 1) throw new Error('Unexpected source node query overflow')
            node.length = input.length - node.offset
        }
        return true
    })
    return Array.from({ length: input.length + 2 }, (_, offset) =>
        [false, true].map((includeRightBound) => {
            const node = parsed.getNodeFromOffset(offset, includeRightBound)
            const index = node == null ? 'None' : `Some(${indices.get(node)})`
            return `    (${rustLiteral(input)}, ${offset}, ${includeRightBound}, ${index}),`
        }),
    ).flat()
})
const fixture = `const CASES: &[(&str, bool, &str, &str)] = &[\n${cases.join('\n')}\n];\nconst QUERY_CASES: &[(&str, usize, bool, Option<usize>)] = &[\n${queries.join('\n')}\n];\n`
await Bun.write(new URL('./json-parser-oracle.rs', import.meta.url), fixture)
process.stdout.write(`${JSON.stringify({ inputs: inputs.size, cases: cases.length, corrections, queries: queries.length })}\n`)
