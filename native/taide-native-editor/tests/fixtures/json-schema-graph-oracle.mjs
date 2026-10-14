import { JSONSchemaService } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js'

import { writeReferenceSource } from './json-schema-reference-source.mjs'
import { createWorkerContext } from './json-schema-worker-context.mjs'
import { rustLiteral } from './emmet-rust-literal.mjs'

const LOCAL_PATCH = new URL('file:///private/tmp/taide-json-schema-graph-local-source-oracle.mjs')
await writeReferenceSource(LOCAL_PATCH)
const CYCLE_PATCH = new URL('file:///private/tmp/taide-json-schema-graph-cycle-source-oracle.mjs')
let source = await Bun.file(LOCAL_PATCH).text()
const replacements = [
    ['let usesUnsupportedFeatures = new Set();', 'const externalReferences = new WeakMap(); let usesUnsupportedFeatures = new Set();'],
    [
        'uri = normalizeId(uri);\n            const referencedHandle = this.getOrAddSchemaHandle(uri);',
        `uri = normalizeId(uri);
            let references = externalReferences.get(node);
            const key = uri + '#' + (refSegment ?? '');
            if (references?.has(key)) { parentHandle.dependencies.add(uri); return this.promise.resolve(undefined); }
            if (!references) { references = new Set(); externalReferences.set(node, references); }
            references.add(key);
            const referencedHandle = this.getOrAddSchemaHandle(uri);`,
    ],
]
for (const [before, after] of replacements) {
    if (source.split(before).length !== 2) throw new Error(`Unexpected graph source marker: ${before}`)
    source = source.replace(before, after)
}
await Bun.write(CYCLE_PATCH, source)
const corrected = await import(CYCLE_PATCH.href)
const { context } = await createWorkerContext(new URL('file:///private/tmp/taide-json-schema-graph-worker-oracle.mjs'))
const MAX_SOURCE_TRAVERSALS = 512
const bounded = (Base) =>
    class extends Base {
        traversals = 0
        traverseNodes(...args) {
            this.traversals += 1
            if (this.traversals > MAX_SOURCE_TRAVERSALS) throw new Error('oracle-cycle-budget')
            return super.traverseNodes(...args)
        }
    }
const ROOT = 'https://schema.example/root.json'
const B = 'https://schema.example/b.json'
const C = 'https://schema.example/c.json'
const cases = [
    [ROOT, {}, true],
    [ROOT, { [ROOT]: 'false' }, true],
    [
        ROOT,
        {
            [ROOT]: '{"type":"object","properties":{"a":{"$ref":"b.json#/definitions/A"},"b":{"$ref":"b.json#/definitions/A"}}}',
            [B]: '{"definitions":{"A":{"type":"string","title":"from B"}}}',
        },
        true,
    ],
    [ROOT, { [ROOT]: '{"$ref":"b.json","title":"local"}', [B]: '{"type":"string","title":"base","id":"skip","$id":"skip"}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/definitions/A"}', [B]: '{"definitions":{"A":{"$ref":"#/definitions/B"},"B":{"type":"integer"}}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#named"}', [B]: '{"definitions":{"A":{"$anchor":"named","type":"number"}}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/definitions/F"}', [B]: '{"definitions":{"F":false}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#na%6Ded"}', [B]: '{"definitions":{"A":{"$anchor":"named","type":"boolean"}}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/missing"}', [B]: '{}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/definitions/A"}', [B]: '\ufeff {"definitions":' }, true],
    [ROOT, { [ROOT]: '\ufeff {"$ref":"b.json"}', [B]: '{"type":"string"}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json"}', [B]: '{"$ref":"root.json","title":"cycle"}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json"}', [B]: '{"$ref":"c.json","title":"B"}', [C]: '{"$ref":"b.json","title":"C"}' }, true],
    [ROOT, { [ROOT]: '{"properties":{"child":{"$ref":"b.json"}}}', [B]: '{"properties":{"child":{"$ref":"root.json"}}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json"}', [B]: '{"$dynamicRef":"#x","$recursiveRef":"#","$dynamicAnchor":"x"}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#name"}', [B]: '{"definitions":{"a":{"$anchor":"name"},"b":{"$anchor":"name","$dynamicAnchor":"n"}}}' }, true],
    [
        'file:///workspace/root.json',
        {
            'file:///workspace/root.json': '{"$ref":"../shared/b.json#/definitions/A"}',
            'file:///shared/b.json': '{"definitions":{"A":{"type":"string"}}}',
        },
        true,
    ],
    ['urn:root', { 'urn:root': '{"$ref":"urn:child"}', 'urn:child': '{"type":"boolean"}' }, false],
    [ROOT, { [ROOT]: '{"$schema":"http://json-schema.org/draft-03/schema#","$ref":"b.json"}', [B]: '{"type":"string"}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/definitions/a~1b"}', [B]: '{"definitions":{"a/b":{"type":"string"}}}' }, true],
    [ROOT, { [ROOT]: '{"$ref":"b.json#/%"}', [B]: '{}' }, true],
]
const units = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const canonical = (value) => {
    const queue = [{ value, parent: -1, edge: '-' }]
    const seen = new Map()
    const parts = []
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const { value, parent, edge } = queue[cursor]
        let kind = typeof value
        let content = ''
        if (value !== null && kind === 'object') {
            if (seen.has(value)) {
                parts.push(`ref:${parent}:${edge}:${seen.get(value)};`)
                continue
            }
            seen.set(value, cursor)
            if (Array.isArray(value)) {
                kind = 'array'
                value.forEach((value, index) => queue.push({ value, parent: cursor, edge: `i${index}` }))
            } else Object.keys(value).forEach((key) => queue.push({ value: value[key], parent: cursor, edge: `k${units(key)}` }))
        } else if (value === null) kind = 'null'
        else if (kind === 'string') content = units(value)
        else content = String(value)
        parts.push(`${kind}:${parent}:${edge}:${content};`)
    }
    return parts.join('')
}
const rows = []
let corrections = 0
for (const [root, documents, workspace] of cases) {
    const run = async (Class) => {
        const service = new (bounded(Class))(
            async (uri) => {
                if (!(uri in documents)) throw `Error: Missing ${uri}.`
                return documents[uri]
            },
            workspace ? context : undefined,
            Promise,
        )
        try {
            const resolved = await service.getOrAddSchemaHandle(root).getResolvedSchema()
            const dependencies = Object.values(service.schemasById)
                .filter((handle) => handle.dependencies.size > 0)
                .map((handle) => `${units(handle.uri)}:${Array.from(handle.dependencies).map(units).join(';')}`)
                .sort()
                .join('|')
            return `${canonical(resolved.schema)}|${resolved.errors.map(units).join(';')}|${resolved.warnings.map(units).join(';')}|${resolved.schemaDraft == null ? '-' : units(resolved.schemaDraft)}|deps:${dependencies}`
        } catch (error) {
            return `throw:${String(error)}`
        }
    }
    const raw = await run(JSONSchemaService)
    const expected = await run(corrected.JSONSchemaService)
    if (raw !== expected) corrections += 1
    rows.push(
        `    (${rustLiteral(root)},&[${Object.entries(documents)
            .map(([uri, content]) => `(${rustLiteral(uri)},${rustLiteral(content)})`)
            .join(',')}],${workspace},${rustLiteral(raw)},${rustLiteral(expected)}),`,
    )
}
await Bun.write(
    new URL('./json-schema-graph-oracle.rs', import.meta.url),
    `const CASES:&[(&str,&[(&str,&str)],bool,&str,&str)]=&[\n${rows.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ cases: rows.length, corrections, cycleGuards: replacements.length })}\n`)
