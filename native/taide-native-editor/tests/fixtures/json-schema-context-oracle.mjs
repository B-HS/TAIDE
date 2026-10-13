import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL('../../../../node_modules/monaco-editor/esm/vs/languages/features/json/jsonWorker.js', import.meta.url)
const PATCH_URL = new URL('file:///private/tmp/taide-json-schema-context-source-oracle.mjs')
const source = await Bun.file(SOURCE_URL).text()
const MARKERS = ['function resolvePath(uriString, path)', 'function normalizePath(parts)', 'function joinPath(uriString, ...paths)']
for (const marker of MARKERS) {
    if (source.split(marker).length !== 2) throw new Error(`Unexpected worker URI marker: ${marker}`)
}
const patched = source.replace(
    /(from\s+|import\s+)'(\.[^']+)'/g,
    (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`,
)
await Bun.write(PATCH_URL, `${patched}\nexport { resolvePath };\n`)
const { JSONWorker, resolvePath } = await import(PATCH_URL.href)
const RESOURCES = [
    '',
    'schema.json',
    '/',
    'file:///workspace/config.json',
    'file:///C:/root/a.json',
    'file://SERVER/root/a.json',
    'https://HOST/path/document.json?x=/y#z',
    'urn:path/file.json',
    'http://User:Pass@HOST:80/path/file.json',
    'file:///漢字/𐐀.json',
    'file:///a%23b/%FF%41/name.json',
    'file:///a/../b/file.json',
    'file:///a\\b/file.json',
    'a b:/root/file.json',
    'file:///\ud800/file.json',
    'file://HOST:80%23%3F/root/file.json',
]
const RELATIVES = [
    '',
    '.',
    '..',
    './schema.json',
    '../schema.json',
    '../../schema.json',
    '/schema.json',
    '//schema.json',
    '/a/../b/',
    '/a/../../../b',
    'a//b/./c/../',
    'a%20b.json',
    '漢字/𐐀.json',
    'a b/#name?query',
    '?q=/path',
    '#name',
    '../%23%3F.json',
    'C:/schema.json',
    'https://host/schema.json',
    '\\schema.json',
    '\ud800',
    'x\udfff',
    'a\nfile.json',
    'a\u2028b.json',
]
const units = (value) => rustLiteral(Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(','))
const contextCases = []
for (const resource of RESOURCES) {
    for (const relative of RELATIVES) {
        let result
        try {
            const base = resource.substr(0, resource.lastIndexOf('/') + 1)
            result = units(resolvePath(base, relative))
        } catch {
            result = 'None'
        }
        if (result !== 'None') result = `Some(${result})`
        contextCases.push(`    (${units(resource)}, ${units(relative)}, ${result}),`)
    }
}
const worker = new JSONWorker(
    { getMirrorModels: () => [] },
    { languageSettings: { validate: true, schemas: [] }, languageId: 'json', enableSchemaRequest: false },
)
const BASIC_INPUTS = [
    '',
    '{}',
    '[]',
    'null',
    '{"$schema":null}',
    '{"$schema":123}',
    '{"$schema":{}}',
    '{"nested":{"$schema":"a.json"}}',
    '{"$schema":null,"$schema":"first.json"}',
    '{"$schema":"first.json","$schema":"second.json"}',
    '{"$schema":,"$schema":"x.json"}',
    '{"$schema":"./x.json","a":"𐐀"}',
    '{"$schema":"./x.json"',
    '{"$schema" "./x.json"}',
    '{"$schema":"../x.json",/*comment*/"x":1}',
]
const PROPERTY_VALUES = [...RELATIVES, 'urn:example:schema', '1foo:x', '_foo:x', '+foo:x', 'é:x', 'a b:x', 'a\n:x', ':x', 'a+b.c-d:x']
const inputs = new Set([...BASIC_INPUTS, ...PROPERTY_VALUES.map((value) => JSON.stringify({ $schema: value }))])
const propertyCases = []
for (const resource of RESOURCES) {
    for (const input of inputs) {
        const document = TextDocument.create(resource, 'json', 1, input)
        const parsed = worker._languageService.parseJSONDocument(document)
        let result
        try {
            const status = worker._languageService.getLanguageStatus(document, parsed)
            result = status.schemas.length > 0 ? `Some(Some(${units(status.schemas[0])}))` : 'Some(None)'
        } catch {
            result = 'None'
        }
        propertyCases.push(`    (${units(resource)}, ${rustLiteral(input)}, ${result}),`)
    }
}
await Bun.write(
    new URL('./json-schema-context-oracle.rs', import.meta.url),
    `const CONTEXT_CASES: &[(&str, &str, Option<&str>)] = &[\n${contextCases.join('\n')}\n];\nconst PROPERTY_CASES: &[(&str, &str, Option<Option<&str>>)] = &[\n${propertyCases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ context: contextCases.length, properties: propertyCases.length, corrections: 0 })}\n`)
