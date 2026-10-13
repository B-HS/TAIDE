import {
    JSONSchemaService,
    UnresolvedSchema,
} from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const PATCH_URL = new URL('file:///private/tmp/taide-json-schema-refs-source-oracle.mjs')
const source = await Bun.file(SOURCE_URL).text()
const replacements = [
    ['path = decodeURIComponent(path);', ''],
    [
        "part = part.replace(/~1/g, '/').replace(/~0/g, '~');",
        "if (/~(?![01])/.test(part)) { current = undefined; return true; } part = part.replace(/~1/g, '/').replace(/~0/g, '~');",
    ],
    [
        'current = current[part];',
        "if (Array.isArray(current) && !/^(0|[1-9][0-9]*)$/.test(part)) { current = undefined; return true; } current = current !== null && typeof current === 'object' && Object.prototype.hasOwnProperty.call(current, part) ? current[part] : undefined;",
    ],
    ['return !current;', 'return current === undefined;'],
    ['const merge = (target, section) => {', 'const merge = (target, section) => { if (section === false) { target.not = {}; return; }'],
    ['section.hasOwnProperty(key)', 'Object.prototype.hasOwnProperty.call(section, key)'],
    [
        'let section;',
        `let section;
            try { if (refSegment !== undefined) refSegment = decodeURIComponent(refSegment); }
            catch { resolveErrors.push(t("$ref '{0}' in '{1}' can not be resolved.", refSegment || '', sourceHandle.uri)); return; }`,
    ],
    ['if (section) {', 'if (section !== undefined) {'],
    [
        'const ref = next.$ref;',
        `if (typeof next.$ref !== 'string') { delete next.$ref; resolveErrors.push('Invalid $ref: a string is required.'); break; } const ref = next.$ref;`,
    ],
]
let patched = source
for (const [before, after] of replacements) {
    if (patched.split(before).length !== 2) throw new Error(`Unexpected original schema refs marker: ${before}`)
    patched = patched.replace(before, after)
}
patched = patched.replace(
    /(from\s+|import\s+)'(\.[^']+)'/g,
    (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`,
)
await Bun.write(PATCH_URL, patched)
const correctedModule = await import(PATCH_URL.href)
const URI = 'file:///schema.json'
const inputs = new Set([
    '{}',
    'true',
    'false',
    '{"type":"string"}',
    '{"$ref":"#"}',
    '{"$ref":"#/definitions/A","definitions":{"A":{"type":"string","title":"A"}}}',
    '{"$ref":"#/definitions/A","title":"local","definitions":{"A":{"type":"string","title":"base","id":"skip","$id":"skip"}}}',
    '{"definitions":{"A":{"type":"string"}},"properties":{"a":{"$ref":"#/definitions/A"},"b":{"$ref":"#/definitions/A"}}}',
    '{"definitions":{"A":{"$ref":"#/definitions/B"},"B":{"type":"number"}},"$ref":"#/definitions/A"}',
    '{"definitions":{"A":{"$ref":"#/definitions/B"},"B":{"$ref":"#/definitions/A"}},"$ref":"#/definitions/A"}',
    '{"properties":{"child":{"$ref":"#"}},"type":"object"}',
    '{"definitions":{"A":{"$anchor":"name","type":"string"}},"$ref":"#name"}',
    '{"definitions":{"A":{"$id":"#name","type":"string"}},"$ref":"#name"}',
    '{"definitions":{"A":{"id":"#name","type":"string"}},"$ref":"#name"}',
    '{"definitions":{"A":{"$id":"","id":"#name","type":"string"}},"$ref":"#name"}',
    '{"definitions":{"A":{"$anchor":"name","type":"string"},"B":{"$anchor":"name","type":"number"}},"$ref":"#name"}',
    '{"$ref":"#/missing"}',
    '{"$ref":"#missing"}',
    '{"$ref":""}',
    '{"$ref":0}',
    '{"$ref":true}',
    '{"definitions":{"A":false},"$ref":"#/definitions/A"}',
    '{"definitions":{"A":true},"$ref":"#/definitions/A"}',
    '{"definitions":{"A":{"hasOwnProperty":0,"type":"string"}},"$ref":"#/definitions/A"}',
    '{"definitions":{"A":{"$anchor":"name","type":"string"}},"$ref":"#na%6De"}',
    '{"definitions":{"A":{"type":"string"}},"$ref":"#%2Fdefinitions%2FA"}',
    '{"definitions":{"a/b":{"type":"string"}},"$ref":"#/definitions/a~1b"}',
    '{"definitions":{"a~b":{"type":"string"}},"$ref":"#/definitions/a~0b"}',
    '{"definitions":{"a~1b":{"type":"string"}},"$ref":"#/definitions/a~01b"}',
    '{"definitions":{"𐐀":{"type":"string"}},"$ref":"#/definitions/%F0%90%90%80"}',
    '{"definitions":{"%2F":{"type":"string"}},"$ref":"#/definitions/%252F"}',
    '{"definitions":{"a~2b":{"type":"string"}},"$ref":"#/definitions/a~2b"}',
    '{"$ref":"#/%"}',
    '{"$ref":"#/%C3"}',
    '{"$ref":"#/%C0%AF"}',
    '{"definitions":{"list":[{"type":"string"}]},"$ref":"#/definitions/list/0"}',
    '{"definitions":{"list":[{"type":"string"}]},"$ref":"#/definitions/list/01"}',
    '{"definitions":{"list":[{"type":"string"}]},"$ref":"#/definitions/list/length"}',
    '{"$ref":"#/constructor"}',
    '{"definitions":{"A":{"$anchor":"a","$recursiveAnchor":true,"$dynamicAnchor":"d","$recursiveRef":"#","$dynamicRef":"#d"}},"$ref":"#a"}',
    '{"$schema":"http://json-schema.org/draft-03/schema#","type":"string"}',
    '{"$schema":"http://json-schema.org/draft-07/schema#","type":"string"}',
    '{"$schema":""}',
])
const NAMES = [
    '',
    'a/b',
    'a~b',
    '~1',
    '~0',
    '~01',
    'a%b',
    '%',
    '漢字',
    '𐐀',
    'é',
    'e\u0301',
    'a b',
    'a#b',
    '0',
    '01',
    '4294967295',
    'hasOwnProperty',
    '__proto__',
    '\u0000',
]
for (const name of NAMES) {
    const escaped = name.replaceAll('~', '~0').replaceAll('/', '~1')
    inputs.add(JSON.stringify({ definitions: { [name]: { type: 'string', title: name } }, $ref: `#/definitions/${encodeURIComponent(escaped)}` }))
    if (name.length > 0) inputs.add(JSON.stringify({ definitions: { A: { $anchor: name, type: 'string' } }, $ref: `#${encodeURIComponent(name)}` }))
}
const SINGLE_FIELDS = [
    'additionalItems',
    'additionalProperties',
    'not',
    'contains',
    'propertyNames',
    'if',
    'then',
    'else',
    'unevaluatedItems',
    'unevaluatedProperties',
]
const MAP_FIELDS = ['definitions', '$defs', 'properties', 'patternProperties', 'dependencies', 'dependentSchemas']
const ARRAY_FIELDS = ['anyOf', 'allOf', 'oneOf', 'prefixItems', 'items']
for (const key of SINGLE_FIELDS) inputs.add(JSON.stringify({ definitions: { A: { type: 'string' } }, [key]: { $ref: '#/definitions/A' } }))
for (const key of MAP_FIELDS)
    inputs.add(JSON.stringify({ definitions: { A: { type: 'string' } }, [key]: { A: { type: 'string' }, B: { $ref: '#/definitions/A' } } }))
for (const key of ARRAY_FIELDS) inputs.add(JSON.stringify({ definitions: { A: { type: 'string' } }, [key]: [{ $ref: '#/definitions/A' }] }))
inputs.add('{"definitions":{"A":{"type":"string"}},"items":{"$ref":"#/definitions/A"}}')
const hexUnits = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const canonical = (result) => {
    const pending = [{ value: result.schema, parent: -1, edge: '-' }]
    const seen = new Map()
    const parts = []
    for (let cursor = 0; cursor < pending.length; cursor += 1) {
        const { value, parent, edge } = pending[cursor]
        let kind = typeof value
        let content = ''
        if (value != null && typeof value === 'object') {
            if (seen.has(value)) {
                parts.push(`ref:${parent}:${edge}:${seen.get(value)};`)
                continue
            }
            seen.set(value, cursor)
            if (Array.isArray(value)) {
                kind = 'array'
                value.forEach((child, index) => pending.push({ value: child, parent: cursor, edge: `i${index}` }))
            } else {
                for (const key of Object.keys(value)) pending.push({ value: value[key], parent: cursor, edge: `k${hexUnits(key)}` })
            }
        } else if (value === null) kind = 'null'
        else if (typeof value === 'string') content = hexUnits(value)
        else content = String(value)
        parts.push(`${kind}:${parent}:${edge}:${content};`)
    }
    return `${parts.join('')}|${result.errors.map(hexUnits).join(';')}|${result.warnings.map(hexUnits).join(';')}|${result.schemaDraft == null ? '-' : hexUnits(result.schemaDraft)}`
}
let corrections = 0
const cases = []
for (const input of inputs) {
    const resolve = async (Service, Unresolved) => {
        const service = new Service(undefined, undefined, Promise)
        return service.resolveSchemaContent(new Unresolved(JSON.parse(input)), { uri: URI, dependencies: new Set() })
    }
    let raw
    try {
        raw = canonical(await resolve(JSONSchemaService, UnresolvedSchema))
    } catch (error) {
        raw = `throw:${String(error)}`
    }
    const expected = canonical(await resolve(correctedModule.JSONSchemaService, correctedModule.UnresolvedSchema))
    if (raw !== expected) corrections += 1
    cases.push(`    (${rustLiteral(input)}, ${rustLiteral(raw)}, ${rustLiteral(expected)}),`)
}
await Bun.write(new URL('./json-schema-refs-oracle.rs', import.meta.url), `const CASES: &[(&str,&str,&str)] = &[\n${cases.join('\n')}\n];\n`)
process.stdout.write(`${JSON.stringify({ cases: cases.length, corrections, sourceCorrections: replacements.length })}\n`)
