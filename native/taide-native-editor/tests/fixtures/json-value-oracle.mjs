import { getNodeValue, parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { equals } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/utils/objects.js'
import { stringLength } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/utils/strings.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const FLOAT_BYTES = 8
const FLOAT_HEX_LENGTH = 16
const SCANNER_ALPHABET = ['{', '}', '[', ']', ':', ',', '"', 'a', '0', ' ', '\n', '/']
const COMBINATION_DEPTH = 3
const floatBytes = new DataView(new ArrayBuffer(FLOAT_BYTES))
const inputs = new Set([''])
let prefixes = ['']
for (let depth = 1; depth <= COMBINATION_DEPTH; depth += 1) {
    prefixes = prefixes.flatMap((prefix) => SCANNER_ALPHABET.map((character) => prefix + character))
    for (const input of prefixes) inputs.add(input)
}
const structured = [
    '{"a":1,"b":[true,false,null,{"c":"value"}]}',
    '{"a":1,"a":2,"a":3,"//":1,"//":2}',
    '{"__proto__":1,"constructor":2,"toString":3,"__proto__":4}',
    '{"b":0,"10":1,"2":2,"0":3,"4294967294":4,"4294967295":5,"01":6,"00":7,"-0":8,"2":9}',
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
    '"\\uD800a"',
    '"\\uD800\\uD800"',
    '"\\uD800\\uDFFF"',
    '"\\uDFFF\\uD800"',
    '"𐐀a"',
]
for (const input of structured) {
    inputs.add(input)
    const characters = Array.from(input)
    for (let length = 1; length < characters.length; length += 1) inputs.add(characters.slice(0, length).join(''))
}
const valueOf = (input) => {
    const document = TextDocument.create('file:///value.json', 'json', 1, input)
    const parsed = parse(document, { collectComments: false })
    return parsed.root == null ? undefined : getNodeValue(parsed.root)
}
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const numberBits = (value) => {
    if (Number.isNaN(value)) return 'nan'
    floatBytes.setFloat64(0, value)
    return floatBytes.getBigUint64(0).toString(16).padStart(FLOAT_HEX_LENGTH, '0')
}
const canonical = (value, corrected) => {
    if (value === undefined) return 'undefined'
    const pending = [{ value, parent: -1, edge: '-' }]
    const parts = []
    for (let cursor = 0; cursor < pending.length; cursor += 1) {
        const { value, parent, edge } = pending[cursor]
        let kind = typeof value
        let content = ''
        let length = '-'
        if (value === null) kind = 'null'
        if (typeof value === 'number') content = numberBits(value)
        if (typeof value === 'boolean') content = String(value)
        if (typeof value === 'string') {
            content = hexUnits(value)
            length = corrected ? Array.from(value).length : stringLength(value)
        }
        if (Array.isArray(value)) {
            kind = 'array'
            value.forEach((child, index) => pending.push({ value: child, parent: cursor, edge: `i${index}` }))
        } else if (value != null && typeof value === 'object') {
            for (const key of Object.keys(value)) pending.push({ value: value[key], parent: cursor, edge: `k${hexUnits(key)}` })
        }
        parts.push(`${kind}:${parent}:${edge}:${content}:${length}:${Boolean(value)};`)
    }
    return parts.join('')
}
let corrections = 0
const cases = Array.from(inputs).map((input) => {
    const value = valueOf(input)
    const raw = canonical(value, false)
    const expected = canonical(value, true)
    if (raw !== expected) corrections += 1
    return `    (${rustLiteral(input)}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`
})
const equalityInputs = [
    'null',
    'true',
    'false',
    '0',
    '-0',
    '1',
    '1.0',
    '1e',
    '1e309',
    '""',
    '"a"',
    '"\\uD800"',
    '[]',
    '{}',
    '[1,2]',
    '[2,1]',
    '{"a":1,"b":[2]}',
    '{"b":[2],"a":1}',
    '{"a":1,"a":2}',
    '{"a":2}',
    '{"a":1e}',
    '[1e]',
    '{"0":0,"1":1}',
    '{"1":1,"0":0}',
]
const equalityCases = equalityInputs.flatMap((one) =>
    equalityInputs.map((two) => `    (${rustLiteral(one)}, ${rustLiteral(two)}, false, ${equals(valueOf(one), valueOf(two))}),`),
)
for (const input of equalityInputs) {
    const value = valueOf(input)
    equalityCases.push(`    (${rustLiteral(input)}, ${rustLiteral(input)}, true, ${equals(value, value)}),`)
}
const fixture = `const CASES: &[(&str, &str, &str)] = &[\n${cases.join('\n')}\n];\nconst EQUALITY_CASES: &[(&str, &str, bool, bool)] = &[\n${equalityCases.join('\n')}\n];\n`
await Bun.write(new URL('./json-value-oracle.rs', import.meta.url), fixture)
process.stdout.write(
    `${JSON.stringify({ inputs: inputs.size, valueCases: cases.length, equalityCases: equalityCases.length, stringLengthCorrections: corrections })}\n`,
)
