import { parse as rawParse } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js'
import { JSONValidation } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonValidation.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'
import { writeBoundedScanner } from './json-source-boundaries.mjs'

const PARSER_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/parser/jsonParser.js',
    import.meta.url,
)
const SCANNER_URL = new URL('../../../../jsonc-parser/lib/esm/impl/scanner.js', PARSER_URL)
const SCANNER_PATCH_URL = new URL('file:///private/tmp/taide-json-validation-scanner-oracle.mjs')
const PARSER_PATCH_URL = new URL('file:///private/tmp/taide-json-validation-source-oracle.mjs')
const SCANNER_MARKER = '{ createScanner, getNodePath'
const ERROR_MARKER = 'function _errorAtRange(message, code, startOffset, endOffset, severity = DiagnosticSeverity.Error) {'
const source = await Bun.file(PARSER_URL).text()
for (const marker of [SCANNER_MARKER, ERROR_MARKER]) {
    if (source.split(marker).length !== 2) throw new Error(`Unexpected original validation parser marker: ${marker}`)
}
await writeBoundedScanner(SCANNER_URL, SCANNER_PATCH_URL)
const errorBoundary = `${ERROR_MARKER}
        startOffset = Math.min(startOffset, text.length);
        endOffset = Math.min(endOffset, text.length);
        if (startOffset > 0 && text.charCodeAt(startOffset) >= 0xdc00 && text.charCodeAt(startOffset) <= 0xdfff) startOffset--;
        if (endOffset < text.length && text.charCodeAt(endOffset) >= 0xdc00 && text.charCodeAt(endOffset) <= 0xdfff) endOffset++;
`
const patched =
    `import { createScanner } from ${JSON.stringify(SCANNER_PATCH_URL.href)};\n` +
    source
        .replace(SCANNER_MARKER, '{ getNodePath')
        .replace(ERROR_MARKER, errorBoundary)
        .replace(/(from\s+|import\s+)'(\.[^']+)'/g, (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, PARSER_URL).href)}`)
await Bun.write(PARSER_PATCH_URL, patched)
const correctedParser = await import(PARSER_PATCH_URL.href)
const LEVELS = [null, 'error', 'warning', 'ignore']
const SCHEMA_POLICIES = [null, { allowComments: true }, { allowTrailingCommas: true }, { allowComments: true, allowTrailingCommas: true }]
const INPUTS = [
    '',
    ' ',
    '{}',
    '[',
    '{',
    'null',
    'true',
    'unknown',
    '1e',
    '1.',
    '01',
    '-',
    '"unterminated',
    '"\\q"',
    '"\\uZZZZ"',
    '"a\u0001b"',
    '{"a":}',
    '{"a":1,}',
    '[1,]',
    '{,}',
    '[,]',
    '{"a" 1}',
    '{"a":1 "b":2}',
    '{"a":1,"a":2,"a":3}',
    '{unquoted:1}',
    '{"//":1,"//":2}',
    '/*',
    '/*x\r\n',
    '//x\r\n{}',
    '{/*x*/"a":0,}',
    '[𐐀',
    '{𐐀',
    '{"𐐀":"\\uD800"}',
    '["\\\n",1]',
    '{"a":1,/*',
    '/*x*/\r\n{/*y*/"a":1,}\r\n//z',
]
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const canonical = (diagnostics, document) =>
    diagnostics
        .map((diagnostic) => {
            const start = document.offsetAt(diagnostic.range.start)
            const end = document.offsetAt(diagnostic.range.end)
            const message = hexUnits(diagnostic.message)
            const source = diagnostic.source == null ? '-' : hexUnits(diagnostic.source)
            return `${diagnostic.code}:${diagnostic.severity}:${start}..${end}:${message}:${source};`
        })
        .join('')
const cases = []
let corrections = 0
for (const input of INPUTS) {
    const document = TextDocument.create('file:///validation.json', 'json', 1, input)
    for (const validate of [false, true]) {
        for (const allowComments of [false, true]) {
            for (const comments of LEVELS) {
                for (const trailingCommas of LEVELS) {
                    for (const schema of SCHEMA_POLICIES) {
                        const schemaService = { getSchemaForResource: async () => (schema == null ? null : { schema, errors: [], warnings: [] }) }
                        const provider = new JSONValidation(schemaService, Promise)
                        provider.configure({ validate, allowComments })
                        const settings = {}
                        if (comments != null) settings.comments = comments
                        if (trailingCommas != null) settings.trailingCommas = trailingCommas
                        const raw = canonical(
                            await provider.doValidation(document, rawParse(document, { collectComments: true }), settings),
                            document,
                        )
                        const expected = canonical(
                            await provider.doValidation(document, correctedParser.parse(document, { collectComments: true }), settings),
                            document,
                        )
                        if (raw !== expected) corrections += 1
                        const commentLevel = comments == null ? 'None' : `Some(Level::${comments.charAt(0).toUpperCase() + comments.slice(1)})`
                        const commaLevel =
                            trailingCommas == null ? 'None' : `Some(Level::${trailingCommas.charAt(0).toUpperCase() + trailingCommas.slice(1)})`
                        const allowSchemaComments = schema?.allowComments === true
                        const allowSchemaCommas = schema?.allowTrailingCommas === true
                        cases.push(
                            `    (${rustLiteral(input)}, ${validate}, ${allowComments}, ${commentLevel}, ${commaLevel}, ${allowSchemaComments}, ${allowSchemaCommas}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`,
                        )
                    }
                }
            }
        }
    }
}
await Bun.write(
    new URL('./json-validation-oracle.rs', import.meta.url),
    `const CASES: &[(&str, bool, bool, Option<Level>, Option<Level>, bool, bool, &str, &str)] = &[\n${cases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ inputs: INPUTS.length, cases: cases.length, corrections })}\n`)
