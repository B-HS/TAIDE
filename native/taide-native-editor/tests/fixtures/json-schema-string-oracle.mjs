import { readFile } from 'node:fs/promises'

import { getNodeValue, parse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js',
    import.meta.url,
)
const FORMAT_COUNT = 8
const source = await readFile(SOURCE_URL, 'utf8')
const formats = Array.from(source.matchAll(/^\s*'([^']+)': \{ errorMessage: t\('([^']+)'\), pattern: \/(.*)\/([i]*) \},$/gm))
if (formats.length !== FORMAT_COUNT) throw new Error('Unexpected JSON format definitions')
const formatRows = formats.map(
    ([, name, message, pattern, flags]) => `    (${rustLiteral(name)}, ${rustLiteral(pattern)}, ${rustLiteral(message)}, ${flags === 'i'}),`,
)
await Bun.write(
    new URL('../../src/json-schema-formats.rs', import.meta.url),
    `const FORMATS: &[(&str, &str, &str, bool)] = &[\n${formatRows.join('\n')}\n];\n`,
)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const cases = []
const add = (value, schema) => {
    const input = JSON.stringify(value)
    const schemaInput = JSON.stringify(schema)
    const document = TextDocument.create('file:///schema-string.json', 'json', 1, input)
    const parsed = parse(document, { collectComments: false })
    const schemaDocument = TextDocument.create('file:///schema.json', 'json', 1, schemaInput)
    const actualSchema = getNodeValue(parse(schemaDocument, { collectComments: false }).root)
    const diagnostics = parsed.validate(document, actualSchema) ?? []
    const expected = diagnostics
        .map((diagnostic) => {
            const start = document.offsetAt(diagnostic.range.start)
            const end = document.offsetAt(diagnostic.range.end)
            if (diagnostic.source !== undefined) throw new Error('Unexpected string diagnostic source')
            return `${diagnostic.code ?? '-'}:${diagnostic.severity}:${Buffer.byteLength(input.slice(0, start))}..${Buffer.byteLength(input.slice(0, end))}:${start}..${end}:${hex(diagnostic.message)};`
        })
        .join('')
    cases.push(`    (${rustLiteral(input)}, ${rustLiteral(schemaInput)}, ${rustLiteral(expected)}),`)
}
const patterns = [
    '',
    '^.$',
    '(?i)^abc$',
    '(?i)^é$',
    '(?i)^k$',
    '(?i)^s$',
    '\\u{10400}',
    '\\p{L}+',
    '\\uD800',
    '\\a',
    '[',
    '[a-',
    '(?i)[',
    '^(a)\\1$',
    'a{1,2}',
    '(?=ab)ab',
    '^\\w+$',
    '^\\u{1F600}$',
    '^𐐀$',
    '^\ud800$',
    '\\u{110000}',
]
const values = ['', 'a', 'aa', 'ab', 'ABC', 'É', 'K', 'ſ', '한글', '𐐀', '\u{1f600}', '\ud800', '\udfff', '\ud800x', '\ud800\udfff', '\n']
for (const pattern of patterns) {
    for (const value of values) add(value, { pattern })
    add('different', { pattern, patternErrorMessage: 'Pattern override \ud800', errorMessage: 'General override' })
    add('different', { pattern, errorMessage: 'General override' })
    add('different', { pattern, patternErrorMessage: '', errorMessage: '' })
}
const formatValues = [
    '',
    'text',
    '#fff',
    '#ffff',
    '#ff00ff',
    '#ff00ffaa',
    '#ff',
    '#ffffffg',
    '2026-10-14',
    '2026-13-14',
    '2026-10-14T12:30:00Z',
    '2026-10-14t23:59:60.12+09:00',
    '23:59:60Z',
    '24:00:00Z',
    'a.b@example.com',
    '"a b"@example.com',
    'bad@example',
    'example.com',
    'EXAMPLE.COM.',
    '-example.com',
    '127.0.0.1',
    '255.255.255.255',
    '256.1.1.1',
    '01.2.3.4',
    '::1',
    '2001:db8::1',
    '::ffff:192.0.2.1',
    '2001:db8::g',
    'https://host/path?x#y',
    '/path',
    '?query',
    '#fragment',
    ':missing',
    'scheme:value',
    '/path:colon',
    'a b:value',
    '\n',
    'a\n:value',
    '\u{1f600}:value',
    '\ud800:value',
]
for (const format of [...formats.map(([, name]) => name), 'uri', 'uri-reference', 'unknown']) {
    for (const value of formatValues) add(value, { format })
    add('invalid', { format, patternErrorMessage: 'Format override', errorMessage: 'General override' })
    add('invalid', { format, errorMessage: 'General override' })
}
for (const value of ['a', 'abc', '한𐐀', '\ud800'])
    add(value, {
        type: 'number',
        const: 0,
        deprecated: true,
        minLength: 2,
        maxLength: 1,
        pattern: '^x$',
        format: 'ipv4',
        errorMessage: 'Common override',
    })
for (const value of [null, true, 1, [], {}]) add(value, { pattern: '[', format: 'uri' })
await Bun.write(new URL('./json-schema-string-oracle.rs', import.meta.url), `const CASES: &[(&str, &str, &str)] = &[\n${cases.join('\n')}\n];\n`)
process.stdout.write(`${JSON.stringify({ formats: formats.length, cases: cases.length })}\n`)
