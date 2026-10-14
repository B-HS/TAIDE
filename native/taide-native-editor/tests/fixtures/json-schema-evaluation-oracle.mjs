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
const { TextDocument } = await import(DOCUMENT_URL.href)
const source = await readFile(SOURCE_URL, 'utf8')
const directory = await mkdtemp(join(tmpdir(), 'taide-json-schema-evaluation-'))
const imports = source.replace(/from (['"])(\.[^'"]+)\1/g, (_, quote, path) => `from ${quote}${new URL(path, SOURCE_URL).href}${quote}`)
const exports = '\nexport { validate, NoOpSchemaCollector, EvaluationContext, getSchemaDraft };\n'
const rawUrl = pathToFileURL(join(directory, 'raw.mjs'))
const correctedUrl = pathToFileURL(join(directory, 'corrected.mjs'))
const LENGTH_CALLS = 2
if (imports.split('stringLength(node.value)').length - 1 !== LENGTH_CALLS) throw new Error('Unexpected JSON string length source')
await Bun.write(rawUrl, imports + exports)
await Bun.write(correctedUrl, imports.replaceAll('stringLength(node.value)', 'Array.from(node.value).length') + exports)
const raw = await import(rawUrl.href)
const corrected = await import(correctedUrl.href)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const rows = []
let corrections = 0
const add = (input, schemaInput) => {
    const document = TextDocument.create('file:///evaluation.json', 'json', 1, input)
    const schemaDocument = TextDocument.create('file:///schema.json', 'json', 1, schemaInput)
    const schema = corrected.getNodeValue(corrected.parse(schemaDocument, { collectComments: false }).root)
    const known = new WeakSet()
    const pending = [schema]
    while (pending.length > 0) {
        const value = pending.pop()
        if (value !== null && typeof value === 'object' && !known.has(value)) {
            known.add(value)
            pending.push(...Object.values(value))
        }
    }
    const evaluate = (service) => {
        const parsed = service.parse(document, { collectComments: false })
        const result = new service.ValidationResult()
        if (parsed.root)
            service.validate(
                parsed.root,
                schema,
                result,
                service.NoOpSchemaCollector.instance,
                new service.EvaluationContext(service.getSchemaDraft(schema)),
            )
        const diagnostics = parsed.validate(document, schema) ?? []
        const diagnosticText = diagnostics
            .map((diagnostic) => {
                const start = document.offsetAt(diagnostic.range.start)
                const end = document.offsetAt(diagnostic.range.end)
                return `${diagnostic.code ?? '-'}:${diagnostic.severity}:${Buffer.byteLength(input.slice(0, start))}..${Buffer.byteLength(input.slice(0, end))}:${start}..${end}:${hex(diagnostic.message)};`
            })
            .join('')
        const processed = Array.from(result.processedProperties).sort().map(hex).join('/')
        const enumValues = result.enumValues === undefined ? '-' : result.enumValues.map((value) => hex(JSON.stringify(value))).join('/')
        const metadata = `${result.propertiesMatches}:${result.propertiesValueMatches}:${result.primaryValueMatches}:${result.enumValueMatch}:${enumValues}:${processed}`
        const matching = parsed
            .getMatchingSchemas(schema)
            .filter((match) => match.schema !== null && typeof match.schema === 'object' && !Array.isArray(match.schema) && known.has(match.schema))
            .map(
                (match) =>
                    `${match.node.offset}..${match.node.offset + match.node.length}:${Boolean(match.inverted)}:${hex(JSON.stringify(match.schema))};`,
            )
            .join('')
        return `${diagnosticText}|${metadata}|${matching}`
    }
    const original = evaluate(raw)
    const expected = evaluate(corrected)
    if (original !== expected) corrections += 1
    rows.push(`    (${rustLiteral(input)}, ${rustLiteral(schemaInput)}, ${rustLiteral(original)}, ${rustLiteral(expected)}),`)
}
const arrays = ['[]', '[1]', '[1,"x",true]', '[1,1]', '[{"a":1},{"a":1}]', '["a","𐐀"]', '[[1],[2]]']
const arraySchemas = [
    { items: { type: 'number' } },
    { prefixItems: [{ type: 'string' }, false], items: false },
    { prefixItems: [true, { const: 0 }], items: { type: 'boolean' } },
    { prefixItems: [{ type: 'number' }], unevaluatedItems: false },
    { items: [{ type: 'string' }], additionalItems: false },
    { items: [{ type: 'number' }, false], additionalItems: { enum: [0] } },
    { contains: { type: 'string' }, minContains: 2, maxContains: 0 },
    { contains: false },
    { contains: { type: 'number' }, unevaluatedItems: false },
    { minItems: 3, maxItems: 1, uniqueItems: true },
    { unevaluatedItems: { const: 0 } },
    { allOf: [{ prefixItems: [true] }], unevaluatedItems: false },
    { anyOf: [{ items: { const: 1 } }, { items: { type: 'string' } }], unevaluatedItems: false },
]
const drafts = [
    undefined,
    'http://json-schema.org/draft-04/schema#',
    'http://json-schema.org/draft-07/schema#',
    'https://json-schema.org/draft/2019-09/schema',
    'https://json-schema.org/draft/2020-12/schema',
]
for (const input of arrays) for (const schema of arraySchemas) for (const draft of drafts) add(input, JSON.stringify({ ...schema, $schema: draft }))
const objects = [
    '{}',
    '{"a":1}',
    '{"a":"x","b":1,"c":true}',
    '{"b":false,"a":0}',
    '{"a":1,"a":"x","b":2}',
    '{"__proto__":1,"constructor":2}',
    '{"𐐀":"한","\\uD800x":0}',
    '{"a":}',
    '{"a":1,"a":}',
]
const objectSchemas = [
    { required: ['a', 'b'], minProperties: 3, maxProperties: 1 },
    { properties: { a: { type: 'number' }, b: false, c: { const: 0 } }, additionalProperties: false },
    { properties: { b: { type: 'string' }, a: false }, additionalProperties: { enum: [null] } },
    { properties: { a: true }, patternProperties: { '^a': false, '(?i)^B$': { const: 0 }, '[': false }, additionalProperties: false },
    { patternProperties: { 'a|b': { type: 'number' }, 'b|c': { const: 0 } }, unevaluatedProperties: false },
    { additionalProperties: true, propertyNames: { minLength: 2 } },
    { dependentRequired: { a: ['b', 'c'], b: ['a'] } },
    { dependentSchemas: { a: { required: ['b'], properties: { b: { const: 0 } } } } },
    { dependencies: { a: { required: ['c'] }, b: ['missing'], c: false } },
    { propertyNames: { enum: ['a', 'b'], deprecated: true } },
    { allOf: [{ properties: { a: { type: 'number' } } }], unevaluatedProperties: false },
    { anyOf: [{ properties: { a: { const: 0 } } }, { properties: { b: { const: 1 } } }], unevaluatedProperties: false },
    { if: { required: ['a'], properties: { a: { const: 1 } } }, then: { required: ['b'] }, else: { required: ['c'] }, unevaluatedProperties: false },
    { required: ['a'], properties: { a: false }, additionalProperties: false, errorMessage: 'Custom \ud800message' },
]
for (const input of objects) for (const schema of objectSchemas) add(input, JSON.stringify(schema))
const primitives = ['null', 'true', '0', '1.0', '"x"', '"\\uD800x"', '[]', '{}', '{"a":0,"b":"x"}']
const combinations = [
    { allOf: [{ type: 'number' }, { const: 0 }, { deprecated: true }], const: true },
    { not: { enum: [null, false, 0] }, errorMessage: 'Not accepted' },
    { anyOf: [{ enum: ['a'] }, { enum: ['b'] }, { const: 2 }] },
    {
        anyOf: [
            { enum: ['a'], errorMessage: 'a only' },
            { enum: ['b'], errorMessage: 'b only' },
        ],
        const: false,
    },
    { oneOf: [{ type: 'number' }, { type: 'integer' }], deprecated: true },
    { oneOf: [true, {}] },
    { allOf: [false, true, { not: false }] },
    { if: { type: 'number' }, then: { minimum: 2 }, else: { type: 'string', minLength: 2 } },
    { if: false, then: false, else: { const: 0 } },
    {
        anyOf: [
            { properties: { a: { const: 0 } }, required: ['a'] },
            { properties: { b: { type: 'number' } }, required: ['b'] },
        ],
    },
    {
        type: 'string',
        allOf: [{ type: 'boolean' }],
        anyOf: [{ const: 1 }, { const: 2 }],
        oneOf: [true, true],
        if: true,
        then: { deprecated: true },
        enum: ['q'],
        const: false,
        deprecated: true,
    },
    { anyOf: [], oneOf: [], allOf: [] },
]
for (const input of primitives) for (const schema of combinations) add(input, JSON.stringify(schema))
add(
    '{"outer":{"a":0}}',
    JSON.stringify({ properties: { outer: { required: ['missing'], properties: { a: { type: 'string' } }, propertyNames: { pattern: '^x$' } } } }),
)
await Bun.write(
    new URL('./json-schema-evaluation-oracle.rs', import.meta.url),
    `const CASES: &[(&str, &str, &str, &str)] = &[\n${rows.join('\n')}\n];\n`,
)
await rm(directory, { recursive: true })
process.stdout.write(`${JSON.stringify({ cases: rows.length, stringLengthCorrections: corrections })}\n`)
