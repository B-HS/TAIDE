import { mkdtemp, rm } from 'node:fs/promises'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const WORKER_URL = new URL('../../../../node_modules/monaco-editor/esm/vs/languages/features/json/jsonWorker.js', import.meta.url)
const REGISTER_URL = new URL('../../../../node_modules/monaco-editor/esm/vs/languages/features/json/register.js', import.meta.url)
const DEFAULTS_MARKER = 'const diagnosticDefault = {'
const remap = (source, url) =>
    source.replace(/(from\s+|import\s+)(['"])(\.[^'"]+)\2/g, (_, prefix, quote, path) => `${prefix}${quote}${new URL(path, url).href}${quote}`)
const directory = await mkdtemp('/private/tmp/taide-json-schema-document-')
const registerSource = await Bun.file(REGISTER_URL).text()
const start = registerSource.indexOf(DEFAULTS_MARKER)
if (start < 0) throw new Error('Unexpected Monaco JSON defaults')
const end = registerSource.indexOf('};', start)
const defaultsUrl = pathToFileURL(join(directory, 'defaults.mjs'))
await Bun.write(defaultsUrl, `${registerSource.slice(start, end + 2)}\nexport { diagnosticDefault };\n`)
const { diagnosticDefault } = await import(defaultsUrl.href)
const workerUrl = pathToFileURL(join(directory, 'worker.mjs'))
await Bun.write(workerUrl, remap(await Bun.file(WORKER_URL).text(), WORKER_URL))
const { JSONWorker } = await import(workerUrl.href)
const hex = (value) => Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(',')
const rows = []

const add = async (uri, input) => {
    const model = { uri: { toString: () => uri }, version: 1, getValue: () => input }
    const worker = new JSONWorker(
        { getMirrorModels: () => [model] },
        { languageSettings: diagnosticDefault, languageId: 'json', enableSchemaRequest: diagnosticDefault.enableSchemaRequest },
    )
    const lines = input.split('\n')
    const offset = (position) => lines.slice(0, position.line).reduce((sum, line) => sum + line.length + 1, 0) + position.character
    const diagnostics = await worker.doValidation(uri)
    const text = diagnostics
        .map((diagnostic) => {
            const code = diagnostic.code === undefined ? '-' : String(diagnostic.code)
            const source = diagnostic.source === undefined ? '-' : hex(diagnostic.source)
            return `${code}:${diagnostic.severity}:${offset(diagnostic.range.start)}..${offset(diagnostic.range.end)}:${hex(diagnostic.message)}:${source};`
        })
        .join('')
    rows.push(`    (${rustLiteral(uri)}, ${rustLiteral(input)}, ${rustLiteral(text)}),`)
}

const uris = ['file:///workspace/a.json', 'file:///workspace/sub/%ED%95%9C.json', 'untitled:3']
const inputs = [
    '{}',
    '{"a":1,}',
    '// c\n{"a":1}',
    '{"$schema":"http://json-schema.org/draft-07/schema#","type":"foo","properties":{"a":{"minimum":"x"}}}',
    '{"$schema":"http://json-schema.org/draft-07/schema","required":"a","items":[1]}',
    '{"$schema":"http://json-schema.org/draft-04/schema#","exclusiveMinimum":1,"type":["string","nope"]}',
    '{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object"}',
    '{"$schema":"https://example.com/schema.json","a":1}',
    '{"$schema":"./local.schema.json","a":1}',
    '{"$schema":"../up/x.json"}',
    '{"$schema":"#frag"}',
    '{"$schema":""}',
    '{"$schema":1,"a":2}',
    '{"a":0,"$schema":"https://example.com/late.json"}',
    '{"$schema":"http://json-schema.org/draft-07/schema#","definitions":{"x":{"$ref":"#/definitions/missing"}},"enum":[]}',
    '{"$schema":"http://json-schema.org/draft-07/schema#","dependencies":{"a":7},"if":{"const":1},"$comment":1}',
    '[{"$schema":"https://example.com/arr.json"}]',
    '{"$schema":"http://json-schema.org/draft-07/schema#",}',
    '{"$schema":"http://json-schema.org/draft-04/schema#","id":5,"properties":{"한":{"type":"strin"}}}',
]
for (const uri of uris) for (const input of inputs) await add(uri, input)
await Bun.write(new URL('./json-schema-document-oracle.rs', import.meta.url), `const CASES: &[(&str, &str, &str)] = &[\n${rows.join('\n')}\n];\n`)
await rm(directory, { recursive: true })
process.stdout.write(`${JSON.stringify({ cases: rows.length })}\n`)
