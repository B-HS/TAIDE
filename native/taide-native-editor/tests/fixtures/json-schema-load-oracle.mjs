import { parse } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/impl/parser.js'
import { JSONSchemaService } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js'
import { URI } from '../../../../node_modules/monaco-editor/esm/external/vscode-uri/lib/esm/index.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const RAW_URL = new URL('../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/impl/parser.js', import.meta.url)
const RAW_PATCH = new URL('file:///private/tmp/taide-json-schema-load-parser-oracle.mjs')
const SERVICE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const SERVICE_PATCH = new URL('file:///private/tmp/taide-json-schema-load-source-oracle.mjs')
const remap = (source, url, map = (url) => url) =>
    source.replace(/(from\s+|import\s+)'(\.[^']+)'/g, (_, prefix, path) => `${prefix}${JSON.stringify(map(new URL(path, url).href))}`)
const rawSource = await Bun.file(RAW_URL).text()
const RAW_MARKER = 'const object = {};'
if (rawSource.split(RAW_MARKER).length !== 2) throw new Error('Unexpected JSONC object marker')
await Bun.write(RAW_PATCH, remap(rawSource.replace(RAW_MARKER, 'const object = Object.create(null);'), RAW_URL))
const correctedRaw = await import(RAW_PATCH.href)
const source = await Bun.file(SERVICE_URL).text()
const SERVICE_MARKER = 'return new UnresolvedSchema(schemaContent, errors);'
if (source.split(SERVICE_MARKER).length !== 2) throw new Error('Unexpected schema content marker')
const validSchema = `if (schemaContent === null || typeof schemaContent !== 'boolean' && (typeof schemaContent !== 'object' || Array.isArray(schemaContent))) {
    if (jsonErrors.length === 0) errors.push(t("Unable to load schema from '{0}': Schema must be an object or boolean.", toDisplayString(url)));
    schemaContent = {};
} `
const URI_URL = new URL('../../../../node_modules/monaco-editor/esm/external/vscode-uri/lib/esm/index.js', import.meta.url)
const URI_WIN_URL = new URL('file:///private/tmp/taide-json-schema-load-windows-uri-oracle.mjs')
const uriSource = await Bun.file(URI_URL).text()
const PLATFORM_MARKER = '"win32"===process.platform'
if (uriSource.split(PLATFORM_MARKER).length !== 2) throw new Error('Unexpected URI platform marker')
await Bun.write(URI_WIN_URL, uriSource.replace(PLATFORM_MARKER, 'true'))
const winUri = await import(URI_WIN_URL.href)
const displayCases = []
await Bun.write(
    SERVICE_PATCH,
    remap(source.replace(SERVICE_MARKER, validSchema + SERVICE_MARKER), SERVICE_URL, (url) =>
        url.endsWith('/jsonc-parser/lib/esm/main.js') ? RAW_PATCH.href : url,
    ),
)
const correctedService = await import(SERVICE_PATCH.href)
const units = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const float = new DataView(new ArrayBuffer(Float64Array.BYTES_PER_ELEMENT))
const describe = (root) => {
    if (root === undefined) return 'undefined'
    const queue = [{ value: root, parent: -1, edge: '-' }]
    const parts = []
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const { value, parent, edge } = queue[cursor]
        let kind = typeof value
        let content = ''
        if (value === null) kind = 'null'
        else if (Array.isArray(value)) {
            kind = 'array'
            value.forEach((value, index) => queue.push({ value, parent: cursor, edge: `i${index}` }))
        } else if (kind === 'object') {
            Object.keys(value).forEach((key) => queue.push({ value: value[key], parent: cursor, edge: `k${units(key)}` }))
        } else if (kind === 'string') content = units(value)
        else if (kind === 'number') {
            float.setFloat64(0, value, false)
            content = float
                .getBigUint64(0, false)
                .toString(16)
                .padStart(Float64Array.BYTES_PER_ELEMENT * 2, '0')
        } else content = String(value)
        parts.push(`${kind}:${parent}:${edge}:${content};`)
    }
    return parts.join('')
}
const inputs = new Set([''])
const ALPHABET = ['{', '}', '[', ']', ':', ',', '"', 'a', '0', ' ', '/', '\n']
const COMBINATION_DEPTH = 3
let prefixes = ['']
for (let depth = 0; depth < COMBINATION_DEPTH; depth += 1) {
    prefixes = prefixes.flatMap((prefix) => ALPHABET.map((unit) => prefix + unit))
    for (const prefix of prefixes) inputs.add(prefix)
}
const structured = [
    '{"type":"object","properties":{"x":{"type":"string"}},"required":["x"]}',
    '{"a":1,"a":2,"//":3,"//":4,"2":5,"1":6,"x":null}',
    '{"__proto__":{"type":"string"},"constructor":0,"hasOwnProperty":false}',
    '{"a":1. ,"b":1e,"c":1e+ ,"d":-0,"e":1e999,"f":1e-999}',
    '{"a" "value","b":2,unquoted:3,"c":}',
    '{"a":1,,"b":2,}',
    '[1,,2,3 4,null,]',
    '{"𐐀":"漢字\\uD800\\uDC00\\uDFFF","b":2}',
    '{"a":"\\q","b":"\\u0","c":"\\\n"}',
    '{ "a":1 } trailing',
    '/* comment */ {"a":true}//comment',
    '/*',
    '/*x\r\n',
    '"unterminated',
    '["𐐀',
    '{"a":1,/*x',
    'true',
    'false',
    'null',
    '1.',
    '1e',
    '1e+',
    '-0',
    '1e999',
    '1e-999',
]
for (const input of structured) {
    inputs.add(input)
    const characters = Array.from(input)
    for (let length = 1; length < characters.length; length += 1) inputs.add(characters.slice(0, length).join(''))
}
const rawCases = []
let rawCorrections = 0
for (const input of inputs) {
    const originalErrors = []
    const errors = []
    const raw = `${describe(parse(input, originalErrors))}|${originalErrors[0]?.offset ?? '-'}`
    const expected = `${describe(correctedRaw.parse(input, errors))}|${errors[0]?.offset ?? '-'}`
    if (raw !== expected) rawCorrections += 1
    rawCases.push(`    (${rustLiteral(input)},${rustLiteral(raw)},${rustLiteral(expected)}),`)
}
const URIS = [
    'file:///workspace/schema.json',
    'file:///C:/root/schema.json',
    'file://HOST/root/a%20b.json',
    'https://host/schema?q#f',
    'http://json-schema.org/draft-07/schema',
    'a b:/schema',
]
const CONTENTS = [
    '',
    ' ',
    '{}',
    'false',
    'true',
    'null',
    '[]',
    '"schema"',
    '1.',
    '@{}',
    '{"a":1,}',
    '{"a":1.}',
    '{"a":1e}',
    '{"a" "x"}',
    '{"__proto__":{"type":"string"}}',
    '{"𐐀":1,/*',
    '\ufeff  {"type":"string"}',
    '\ufeff\u00a0\u2028[',
    '\ufeff',
    '/*',
    ...structured,
]
const loadCases = []
let loadCorrections = 0
for (const uri of URIS) {
    for (const content of new Set(CONTENTS)) {
        const run = async (Class) => {
            const service = new Class(async () => content)
            const value = await service.loadSchema(uri)
            return `${describe(value.schema)}|${value.errors.map(units).join(';')}`
        }
        const raw = await run(JSONSchemaService)
        const expected = await run(correctedService.JSONSchemaService)
        if (raw !== expected) loadCorrections += 1
        loadCases.push(`    (${rustLiteral(uri)},ResponseInput::Text(${rustLiteral(content)}),${rustLiteral(raw)},${rustLiteral(expected)}),`)
    }
    for (const failure of ['Error: owned failure.', 'TypeError: outer Error: inner.', 'owned failure..', '', '漢字𐐀.']) {
        const run = async (Class) => {
            const value = await new Class(async () => {
                throw failure
            }).loadSchema(uri)
            return `${describe(value.schema)}|${value.errors.map(units).join(';')}`
        }
        const raw = await run(JSONSchemaService)
        const expected = await run(correctedService.JSONSchemaService)
        loadCases.push(`    (${rustLiteral(uri)},ResponseInput::Failed(${rustLiteral(failure)}),${rustLiteral(raw)},${rustLiteral(expected)}),`)
    }
    const value = await new JSONSchemaService().loadSchema(uri)
    const expected = `${describe(value.schema)}|${value.errors.map(units).join(';')}`
    loadCases.push(`    (${rustLiteral(uri)},ResponseInput::Unavailable,${rustLiteral(expected)},${rustLiteral(expected)}),`)
}
for (const input of [...URIS, 'file://HOST', 'file:///漢字/𐐀.json', 'file:///C:/a%23b.json']) {
    for (const [isWindows, Uri] of [
        [false, URI],
        [true, winUri.URI],
    ]) {
        let display = input
        try {
            const parsed = Uri.parse(input)
            if (parsed.scheme === 'file') display = parsed.fsPath
        } catch {}
        displayCases.push(`    (${rustLiteral(input)},${isWindows},${rustLiteral(units(display))}),`)
    }
}
await Bun.write(
    new URL('./json-schema-load-oracle.rs', import.meta.url),
    `const RAW_CASES:&[(&str,&str,&str)]=&[\n${rawCases.join('\n')}\n];\nconst LOAD_CASES:&[(&str,ResponseInput,&str,&str)]=&[\n${loadCases.join('\n')}\n];\nconst DISPLAY_CASES:&[(&str,bool,&str)]=&[\n${displayCases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ raw: rawCases.length, rawCorrections, loads: loadCases.length, loadCorrections })}\n`)
