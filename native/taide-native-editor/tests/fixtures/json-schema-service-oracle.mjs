import { JSONSchemaService } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js'
import { TextDocument } from '../../../../node_modules/monaco-editor/esm/external/vscode-languageserver-textdocument/lib/esm/main.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SERVICE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const SERVICE_PATCH = new URL('file:///private/tmp/taide-json-schema-service-source-oracle.mjs')
let serviceSource = await Bun.file(SERVICE_URL).text()
const REPLACEMENTS = [
    ['if (unresolvedSchemaContent) {', 'if (unresolvedSchemaContent !== undefined) {'],
    [
        'return config.schema ? this.addSchemaHandle(id, config.schema) : this.getOrAddSchemaHandle(id);',
        'return config.schema !== undefined ? this.addSchemaHandle(id, config.schema) : this.getOrAddSchemaHandle(id);',
    ],
]
for (const [before, after] of REPLACEMENTS) {
    if (serviceSource.split(before).length !== 2) throw new Error(`Unexpected original registry marker: ${before}`)
    serviceSource = serviceSource.replace(before, after)
}
const remap = (source, url) =>
    source.replace(/(from\s+|import\s+)'(\.[^']+)'/g, (_, prefix, path) => `${prefix}${JSON.stringify(new URL(path, url).href)}`)
await Bun.write(SERVICE_PATCH, `${remap(serviceSource, SERVICE_URL)}\nexport { normalizeId };\n`)
const corrected = await import(SERVICE_PATCH.href)
const WORKER_URL = new URL('../../../../node_modules/monaco-editor/esm/vs/languages/features/json/jsonWorker.js', import.meta.url)
const WORKER_PATCH = new URL('file:///private/tmp/taide-json-schema-service-worker-oracle.mjs')
const workerSource = await Bun.file(WORKER_URL).text()
const WORKER_MARKER = 'this._languageService = getLanguageService({'
if (workerSource.split(WORKER_MARKER).length !== 2) throw new Error('Unexpected original worker context marker')
await Bun.write(
    WORKER_PATCH,
    `${remap(workerSource.replace(WORKER_MARKER, 'this._languageService = captureLanguageService({'), WORKER_URL)}\nlet capturedContext;\nconst captureLanguageService = (params) => { capturedContext=params.workspaceContext; return getLanguageService(params); };\nexport { capturedContext };\n`,
)
const workerModule = await import(WORKER_PATCH.href)
const worker = new workerModule.JSONWorker(
    { getMirrorModels: () => [] },
    { languageSettings: { schemas: [] }, languageId: 'json', enableSchemaRequest: false },
)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const describe = (root) => {
    const queue = [{ value: root, parent: -1, edge: '-' }]
    const result = []
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const { value, parent, edge } = queue[cursor]
        let kind = typeof value
        let content = ''
        if (value === undefined) {
            kind = 'undefined'
        } else if (value === null) {
            kind = 'null'
        } else if (Array.isArray(value)) {
            kind = 'array'
            value.forEach((value, index) => queue.push({ value, parent: cursor, edge: `i${index}` }))
        } else if (typeof value === 'object') {
            kind = 'object'
            Object.keys(value).forEach((key) => queue.push({ value: value[key], parent: cursor, edge: `k${hex(key)}` }))
        } else if (typeof value === 'string') {
            content = hex(value)
        } else {
            content = String(value)
        }
        result.push(`${kind}:${parent}:${edge}:${content};`)
    }
    return result.join('')
}
const reg = (id, schema, patterns = [], folder = null, contribution = false) => ({ kind: 'Reg', id, schema, patterns, folder, contribution })
const action = (kind, id = null, document = null) => ({ kind, id, document })
const CASES = [
    [
        reg('urn:base', '{"title":"base"}', ['*.json'], null, true),
        action('Ids'),
        action('Clear'),
        action('Ids'),
        action('Uris', 'file:///a.json'),
        action('Load', 'urn:base'),
        reg('urn:base', '{"title":"external"}', ['*.json']),
        action('Load', 'urn:base'),
        action('Clear'),
        action('Load', 'urn:base'),
        action('Resolve', 'urn:base'),
        action('Resolve', 'urn:base'),
        action('Change', 'urn:base'),
        action('Load', 'urn:base'),
    ],
    [
        reg('HTTP://HOST/a#', '{"type":"string"}', ['*.json']),
        reg('urn:b', 'false', ['*.json']),
        reg('HTTP://HOST/a', null, ['*.yaml']),
        action('Ids'),
        action('Uris', 'file:///a.json?x#y'),
        action('Combined', "file:///a'(!).json?q#f"),
        action('Uris', 'file:///a.json', '{"$schema":"../custom.json"}'),
        action('Uris', 'file:///a.json', '{"$schema":"urn:first","$schema":"urn:last"}'),
        action('Load', 'urn:b'),
        action('Resolve', 'urn:b'),
    ],
    [
        reg('urn:a', '{}', ['*.json']),
        reg('urn:a', null, ['!a.json', 'b.json']),
        reg('schemaservice://internal/x', '{}', ['*.json']),
        action('Ids'),
        action('Uris', 'file:///a.json'),
        action('Uris', 'file:///b.json'),
        action('Uris', 'file:///b.json', '{}'),
        action('Uris', 'file:///b.json', '{"$schema":null}'),
        action('Uris', 'file:///b.json', '{"$schema":""}'),
    ],
    [
        reg('urn:a', 'true', ['*.json'], 'file:///workspace'),
        reg('urn:b', '{"title":"b"}', ['*.json'], 'file:///workspace2'),
        action('Uris', 'file:///workspace/a.json'),
        action('Uris', 'file:///workspace2/a.json'),
        action('Uris', 'file:///other/a.json'),
        action('Load', 'urn:a'),
        action('Resolve', 'urn:a'),
        action('Change', 'urn:unknown'),
        action('Resolve', 'urn:a'),
    ],
    [
        action('Load', 'urn:load'),
        action('Load', 'urn:load#'),
        action('Resolve', 'urn:load'),
        action('Resolve', 'urn:load'),
        action('Change', 'urn:load'),
        action('Load', 'urn:load'),
        action('Change', 'urn:load'),
        action('Change', 'urn:load'),
        action('Clear'),
        action('Ids'),
    ],
    [
        reg('urn:base', 'false', [], null, true),
        action('Clear'),
        action('Load', 'urn:base'),
        reg('urn:base', 'true'),
        action('Resolve', 'urn:base'),
        action('Clear'),
        action('Resolve', 'urn:base'),
        action('Change', 'urn:base'),
        action('Load', 'urn:base'),
    ],
]
const run = async (Class, events) => {
    let requests = 0
    const service = new Class(
        async (uri) => {
            requests += 1
            return JSON.stringify({ title: `loaded:${uri}` })
        },
        workerModule.capturedContext,
        Promise,
    )
    const results = []
    for (const event of events) {
        let result = 'ok'
        if (event.kind === 'Reg') {
            const schema = event.schema == null ? undefined : JSON.parse(event.schema)
            if (event.contribution) {
                service.setSchemaContributions({
                    schemas: { [event.id]: schema },
                    schemaAssociations: event.patterns.length > 0 ? [{ pattern: event.patterns, folderUri: event.folder, uris: [event.id] }] : [],
                })
            } else {
                service.registerExternalSchema({ uri: event.id, schema, fileMatch: event.patterns, folderUri: event.folder })
            }
        }
        if (event.kind === 'Clear') service.clearExternalSchemas()
        if (event.kind === 'Ids') result = `ids:${service.getRegisteredSchemaIds().map(hex).join(';')}`
        if (event.kind === 'Uris') {
            const document =
                event.document == null
                    ? undefined
                    : worker._languageService.parseJSONDocument(TextDocument.create(event.id, 'json', 1, event.document))
            result = `uris:${service.getSchemaURIsForResource(event.id, document).map(hex).join(';')}`
        }
        if (event.kind === 'Combined') {
            const ids = service.getAssociatedSchemas(event.id)
            const handle = service.createCombinedSchema(event.id, ids)
            const unresolved = await handle.getUnresolvedSchema()
            result = `combined:${hex(handle.uri)}|${describe(unresolved.schema)}`
        }
        if (event.kind === 'Load' || event.kind === 'Resolve') {
            const handle = service.getOrAddSchemaHandle(corrected.normalizeId(event.id))
            const value = event.kind === 'Load' ? await handle.getUnresolvedSchema() : await handle.getResolvedSchema()
            result = `${event.kind.toLowerCase()}:${describe(value.schema)}|errors:${value.errors.length}|requests:${requests}`
        }
        if (event.kind === 'Change') result = `change:${service.onResourceChange(event.id)}`
        results.push(result)
    }
    return results
}
let corrections = 0
const rows = []
for (const events of CASES) {
    const raw = await run(JSONSchemaService, events)
    const expected = await run(corrected.JSONSchemaService, events)
    const actions = events.map((event) => {
        if (event.kind === 'Reg')
            return `Action::Reg(${rustLiteral(event.id)},${event.schema == null ? 'None' : `Some(${rustLiteral(event.schema)})`},&[${event.patterns.map(rustLiteral).join(',')}],${event.folder == null ? 'None' : `Some(${rustLiteral(event.folder)})`},${event.contribution})`
        if (event.kind === 'Uris')
            return `Action::Uris(${rustLiteral(event.id)},${event.document == null ? 'None' : `Some(${rustLiteral(event.document)})`})`
        if (event.id == null) return `Action::${event.kind}`
        return `Action::${event.kind}(${rustLiteral(event.id)})`
    })
    corrections += expected.filter((value, index) => value !== raw[index]).length
    rows.push(`    (&[${actions.join(',')}],&[${raw.map(rustLiteral).join(',')}],&[${expected.map(rustLiteral).join(',')}]),`)
}
await Bun.write(
    new URL('./json-schema-service-oracle.rs', import.meta.url),
    `const CASES:&[(&[Action],&[&str],&[&str])]=&[\n${rows.join('\n')}\n];\n`,
)
process.stdout.write(
    `${JSON.stringify({ scenarios: CASES.length, events: CASES.reduce((count, events) => count + events.length, 0), corrections })}\n`,
)
