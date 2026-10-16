import { rustLiteral } from './emmet-rust-literal.mjs'

const SERVICE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/jsonLanguageService.js',
    import.meta.url,
)
const DOCUMENT_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js',
    import.meta.url,
)
const SCHEMA_URI = 'file:///schema.json'
const DOCUMENT_URI = 'file:///doc.json'
const { getLanguageService } = await import(SERVICE_URL.href)
const { TextDocument } = await import(DOCUMENT_URL.href)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const rows = []

const schemas = [
    null,
    { type: 'object', properties: { a: { type: 'number' } }, required: ['b'] },
    { $schema: 'http://json-schema.org/draft-03/schema#', type: 'string' },
    { $defs: { x: { $recursiveAnchor: true } }, $dynamicAnchor: 'd', type: 'string' },
    { allowComments: true, allOf: [{ allowTrailingCommas: true }] },
    { allowComments: 'x', allOf: [{ allowsTrailingCommas: true }, { allowComments: false, allowTrailingCommas: false }] },
    { $ref: '#/definitions/missing' },
    { $id: '#dup', $defs: { a: { $anchor: 'dup' } }, type: 'array' },
    { properties: { a: { enum: [1, 2] }, b: { deprecationMessage: 'old' } } },
    { allOf: [{ type: 'string' }, { type: 'string' }] },
    { items: [{ type: 'number' }], additionalItems: false, $schema: 'http://json-schema.org/draft-04/schema#' },
    { prefixItems: [{ type: 'number' }], items: false },
    { $ref: '#a', $defs: { a: { $anchor: 'a', $recursiveAnchor: true, type: 'number' } }, $dynamicAnchor: 'd' },
]
const documents = [
    '{"a":"x"}',
    `{"$schema":"${SCHEMA_URI}","a":1,}`,
    '// c\n{"a":1 /* d */}',
    '[1,]',
    '',
    '{"a":"한𐐀",\n"b":2}',
    '"str"',
    `{"$schema":"${SCHEMA_URI}"}`,
    '{"$schema"}',
    '[1,"x"]',
]
const settings = [
    [{ validate: true, allowComments: false }, undefined],
    [
        { validate: true, allowComments: true },
        { trailingCommas: 'warning', comments: 'ignore' },
    ],
    [{ validate: true }, { schemaValidation: 'error', schemaRequest: 'error' }],
    [{ validate: true }, { schemaValidation: 'ignore' }],
    [{ validate: true }, { schemaRequest: 'ignore', comments: 'warning' }],
    [{ validate: true }, { schemaValidation: 'warning', schemaDraft: 4 }],
    [{ validate: false }, undefined],
]
const describe = (document, diagnostics) =>
    diagnostics
        .map((diagnostic) => {
            const start = document.offsetAt(diagnostic.range.start)
            const end = document.offsetAt(diagnostic.range.end)
            const code = diagnostic.code === undefined ? '-' : String(diagnostic.code)
            const source = diagnostic.source === undefined ? '-' : hex(diagnostic.source)
            return `${code}:${diagnostic.severity}:${start}..${end}:${hex(diagnostic.message)}:${source};`
        })
        .join('')

for (const schema of schemas) {
    // The language service resolves $ref in place, so every service receives a fresh copy.
    const schemaText = schema === null ? '' : JSON.stringify(schema)
    for (const input of documents) {
        // A $schema property without registered schemas selects a remote load, which belongs to the service path.
        if (schema === null && input.includes('$schema')) continue
        for (const [configuration, documentSettings] of settings) {
            const service = getLanguageService({})
            service.configure({
                ...configuration,
                schemas: schema === null ? [] : [{ uri: SCHEMA_URI, fileMatch: ['*.json'], schema: JSON.parse(schemaText) }],
            })
            const document = TextDocument.create(DOCUMENT_URI, 'json', 1, input)
            const diagnostics = await service.doValidation(document, service.parseJSONDocument(document), documentSettings)
            rows.push(
                `    (${rustLiteral(schemaText)}, ${rustLiteral(input)}, ${rustLiteral(JSON.stringify(configuration))}, ${rustLiteral(JSON.stringify(documentSettings ?? {}))}, ${rustLiteral(describe(document, diagnostics))}),`,
            )
        }
    }
}
await Bun.write(
    new URL('./json-schema-policy-oracle.rs', import.meta.url),
    `const CASES: &[(&str, &str, &str, &str, &str)] = &[\n${rows.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ cases: rows.length })}\n`)
