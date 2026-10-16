import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js',
    import.meta.url,
)
const DOCUMENT_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js',
    import.meta.url,
)
const NO_SELECTION = -1
const LENGTH_CALLS = 2
const { TextDocument } = await import(DOCUMENT_URL.href)
const source = await readFile(SOURCE_URL, 'utf8')
const directory = await mkdtemp(join(tmpdir(), 'taide-json-schema-matching-'))
const imports = source.replace(/from (['"])(\.[^'"]+)\1/g, (_, quote, path) => `from ${quote}${new URL(path, SOURCE_URL).href}${quote}`)
if (imports.split('stringLength(node.value)').length - 1 !== LENGTH_CALLS) throw new Error('Unexpected JSON string length source')
const correctedUrl = pathToFileURL(join(directory, 'corrected.mjs'))
await Bun.write(correctedUrl, imports.replaceAll('stringLength(node.value)', 'Array.from(node.value).length'))
const service = await import(correctedUrl.href)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const rows = []

const describeSchema = (schema, known) => {
    if (typeof schema === 'boolean') return `S:${hex(JSON.stringify(schema))}`
    if (known.has(schema)) return `S:${hex(JSON.stringify(schema))}`
    const text = JSON.stringify(schema)
    if (text === '{}') return 'E'
    if (text === '{"not":{}}') return 'R'
    throw new Error(`Unknown temporary schema ${text}`)
}

const add = (input, schemaInput) => {
    const document = TextDocument.create('file:///matching.json', 'json', 1, input)
    const schemaDocument = TextDocument.create('file:///schema.json', 'json', 1, schemaInput)
    const schema = service.getNodeValue(service.parse(schemaDocument, { collectComments: false }).root)
    const known = new WeakSet()
    const pending = [schema]
    while (pending.length > 0) {
        const value = pending.pop()
        if (value !== null && typeof value === 'object' && !known.has(value)) {
            known.add(value)
            pending.push(...Object.values(value))
        }
    }
    const parsed = service.parse(document, { collectComments: false })
    const offsets = new Set([NO_SELECTION, input.length])
    const visit = (node) => {
        if (!node) return
        offsets.add(node.offset)
        offsets.add(node.offset + 1)
        for (const child of node.children ?? []) visit(child)
    }
    visit(parsed.root)
    const sorted = Array.from(offsets).sort((left, right) => left - right)
    const run = (focus, excludeOffset) => {
        const exclude = excludeOffset === NO_SELECTION ? undefined : parsed.getNodeFromOffset(excludeOffset)
        const matches = parsed.getMatchingSchemas(schema, focus, exclude)
        const text = matches
            .map(
                (match) =>
                    `${match.node.offset}..${match.node.offset + match.node.length}:${Boolean(match.inverted)}:${describeSchema(match.schema, known)};`,
            )
            .join('')
        const excluded = exclude === undefined ? '-' : `${exclude.type}@${exclude.offset}..${exclude.offset + exclude.length}`
        rows.push(
            `    (${rustLiteral(input)}, ${rustLiteral(schemaInput)}, ${focus}, ${excludeOffset}, ${rustLiteral(excluded)}, ${rustLiteral(text)}),`,
        )
    }
    for (const focus of sorted) run(focus, NO_SELECTION)
    for (const exclude of sorted) if (exclude !== NO_SELECTION) run(NO_SELECTION, exclude)
    run(sorted[Math.min(2, sorted.length - 1)], sorted[Math.min(3, sorted.length - 1)])
}

const documents = ['{"a":1,"b":[true,"x"]}', '[1,{"k":"한"},null]', '"𐐀"', '{"a":{"b":{"c":0}}}', '{"a":}', '7']
const schemas = [
    true,
    false,
    {},
    { properties: { a: true, b: false } },
    { properties: { a: { type: 'number' }, b: { items: [true, false], additionalItems: { const: 0 } } } },
    { prefixItems: [true, false], items: { properties: { k: false } } },
    { items: { anyOf: [{ type: 'number' }, true, false] } },
    { allOf: [true, false, { not: true }, { not: false }] },
    { anyOf: [false, { type: 'string' }], oneOf: [true, { const: 7 }] },
    { if: true, then: false, else: true, not: { if: false, then: true, else: false } },
    { properties: { a: { properties: { b: { properties: { c: { const: 1 } } } } } }, dependencies: { a: true } },
    { unevaluatedItems: true, unevaluatedProperties: { not: false }, additionalProperties: { anyOf: [true] } },
    { contains: false, propertyNames: false, patternProperties: { '^a': { allOf: [true] } } },
    { $schema: 'http://json-schema.org/draft-04/schema#', items: [true, { type: 'string' }], additionalItems: true },
]
for (const input of documents) for (const schema of schemas) add(input, JSON.stringify(schema))
await Bun.write(
    new URL('./json-schema-matching-oracle.rs', import.meta.url),
    `const CASES: &[(&str, &str, isize, isize, &str, &str)] = &[\n${rows.join('\n')}\n];\n`,
)
await rm(directory, { recursive: true })
process.stdout.write(`${JSON.stringify({ cases: rows.length })}\n`)
