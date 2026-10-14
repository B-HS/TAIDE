import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { getNodeValue, parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js',
    import.meta.url,
)
const EXPECTED_LENGTH_CALLS = 2
const ERROR_SEVERITY = 1
const WARNING_SEVERITY = 2
const source = await readFile(SOURCE_URL, 'utf8')
if (source.split('stringLength(node.value)').length - 1 !== EXPECTED_LENGTH_CALLS) throw new Error('Unexpected JSON string length source')
const directory = await mkdtemp(join(tmpdir(), 'taide-json-schema-scalar-'))
const correctedUrl = pathToFileURL(join(directory, 'parser.mjs'))
const correctedSource = source
    .replaceAll('stringLength(node.value)', 'Array.from(node.value).length')
    .replace(/from (['"])(\.[^'"]+)\1/g, (_, quote, path) => `from ${quote}${new URL(path, SOURCE_URL).href}${quote}`)
await Bun.write(correctedUrl, correctedSource)
const corrected = await import(correctedUrl.href)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const rows = []
let corrections = 0
const add = (input, schemaInput, severity = WARNING_SEVERITY, property = false) => {
    const document = TextDocument.create('file:///schema-scalar.json', 'json', 1, input)
    const originalDocument = parse(document, { collectComments: false })
    const correctedDocument = corrected.parse(document, { collectComments: false })
    const schemaDocument = TextDocument.create('file:///schema.json', 'json', 1, schemaInput)
    const schema = getNodeValue(parse(schemaDocument, { collectComments: false }).root)
    const fullSchema = property ? { properties: { target: schema } } : schema
    const selected = property ? originalDocument.root.properties.find((entry) => entry.keyNode.value === 'target').valueNode : originalDocument.root
    const describe = (diagnostics) =>
        (diagnostics ?? [])
            .map((diagnostic) => {
                const start = document.offsetAt(diagnostic.range.start)
                const end = document.offsetAt(diagnostic.range.end)
                const bytesStart = Buffer.byteLength(input.slice(0, start))
                const bytesEnd = Buffer.byteLength(input.slice(0, end))
                if (diagnostic.source !== undefined) throw new Error('Unexpected scalar diagnostic source')
                return `${diagnostic.code ?? '-'}:${diagnostic.severity}:${bytesStart}..${bytesEnd}:${start}..${end}:${hex(diagnostic.message)};`
            })
            .join('')
    const raw = describe(originalDocument.validate(document, fullSchema, severity))
    const expected = describe(correctedDocument.validate(document, fullSchema, severity))
    if (raw !== expected) corrections += 1
    const targetByte = Buffer.byteLength(input.slice(0, selected.offset))
    rows.push(`    (${rustLiteral(input)}, ${rustLiteral(schemaInput)}, ${targetByte}, ${severity}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`)
}
const values = ['null', 'true', 'false', '0', '-0', '1', '1.0', '1e2', '1.5', '""', '"text"', '[]', '[1,2]', '{}', '{"a":1,"b":[2]}']
const types = [
    'null',
    'boolean',
    'number',
    'integer',
    'string',
    'array',
    'object',
    'unknown',
    '',
    ['string', 'integer'],
    ['array', 'object'],
    [],
    null,
    false,
]
for (const input of values) {
    for (const type of types) add(input, JSON.stringify({ type }))
    for (const schema of [
        { enum: [] },
        { enum: [null, false, 0, 'text', { b: [2], a: 1 }] },
        { enum: [null], const: false, deprecated: true },
        { const: [1, 2] },
        { type: 'string', enum: [0], const: true, deprecationMessage: 'Use the replacement' },
        { type: 'array', enum: ['x'], const: null, errorMessage: 'Custom \ud800message', deprecated: true },
        { type: ['number', 'boolean'], errorMessage: '', deprecationMessage: '' },
    ])
        add(input, JSON.stringify(schema), ERROR_SEVERITY)
}
const numbers = ['-1', '-0', '0', '1', '1.0', '1.5', '0.3', '0.6', '1e-7', '1e21', '9007199254740993', '1e309', '-1e309']
const divisors = ['0', '0.1', '0.2', '0.3', '2', '-0.1', '1e-7', '1e21', '1e309']
for (const input of numbers) {
    for (const divisor of divisors) add(input, `{"multipleOf":${divisor}}`)
    for (const schema of [
        { minimum: 0, maximum: 1 },
        { minimum: 0, maximum: 1, exclusiveMinimum: true, exclusiveMaximum: true },
        { minimum: 2, maximum: -2, exclusiveMinimum: 3, exclusiveMaximum: -3, multipleOf: 2, deprecated: true },
        { minimum: 0, maximum: 1, exclusiveMinimum: false, exclusiveMaximum: false },
        { multipleOf: 0.2, minimum: 2, maximum: -2, errorMessage: 'Only common errors use this', const: 9, type: 'boolean' },
    ])
        add(input, JSON.stringify(schema))
}
const strings = ['""', '"a"', '"ab"', '"𐐀한"', '"\\uD800x"', '"\\uDFFFx"', '"\\uD800\\uD800"', '"\\uD800\\uDFFFx"', '"\\b\\t\\n\\f\\r\\u0000"']
for (const input of strings) {
    for (const schema of [
        { minLength: 2, maxLength: 1 },
        { minLength: 1.5, maxLength: -1 },
        { minLength: 2, errorMessage: 'Not used for length' },
    ]) {
        add(input, JSON.stringify(schema))
    }
    add(`{"앞𐐀":0,"target":${input}}`, JSON.stringify({ type: 'number', enum: [0], deprecated: true, minLength: 2 }), ERROR_SEVERITY, true)
}
const stringifyInputs = [
    ...values,
    ...numbers,
    ...strings,
    '1000000000000000100',
    '1.0000000000000001e18',
    '1e23',
    '2.2250738585072014e-308',
    '5e-324',
    '1.7976931348623157e308',
    '0.10000000000000002',
    '{"b":0,"10":1,"2":2,"0":3,"4294967294":4,"4294967295":5,"01":6,"__proto__":7}',
    '{"\\uD800key":"\\uDFFF","paired":"\\uD800\\uDFFF","control":"\\u0001\\\"\\\\/"}',
]
const stringifyRows = stringifyInputs.map((input) => {
    const document = TextDocument.create('file:///stringify.json', 'json', 1, input)
    const value = getNodeValue(parse(document, { collectComments: false }).root)
    return `    (${rustLiteral(input)}, ${rustLiteral(hex(JSON.stringify(value)))}),`
})
const fixture = `const CASES: &[(&str, &str, usize, u8, &str, &str)] = &[\n${rows.join('\n')}\n];\nconst STRINGIFY_CASES: &[(&str, &str)] = &[\n${stringifyRows.join('\n')}\n];\n`
await Bun.write(new URL('./json-schema-scalar-oracle.rs', import.meta.url), fixture)
await rm(directory, { recursive: true })
process.stdout.write(`${JSON.stringify({ scalarCases: rows.length, stringifyCases: stringifyRows.length, stringLengthCorrections: corrections })}\n`)
