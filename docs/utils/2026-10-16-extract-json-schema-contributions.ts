// Extracts the installed vscode-json-languageservice schema contributions (MIT) into the native editor resources.
const SOURCE = '../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/configuration.js'
const TARGET = new URL('../../native/taide-native-editor/resources/', import.meta.url)
const INDENT = 2
const FILES = {
    'http://json-schema.org/draft-04/schema#': 'json-schema-draft-04.json',
    'http://json-schema.org/draft-07/schema#': 'json-schema-draft-07.json',
}

const { schemaContributions } = await import(new URL(SOURCE, import.meta.url).href)
if (schemaContributions.schemaAssociations.length !== 0) throw new Error('Unexpected schema associations')
const uris = Object.keys(schemaContributions.schemas)
if (uris.join() !== Object.keys(FILES).join()) throw new Error(`Unexpected schema contributions ${uris.join()}`)
for (const uri of uris) await Bun.write(new URL(FILES[uri], TARGET), `${JSON.stringify(schemaContributions.schemas[uri], null, INDENT)}\n`)
process.stdout.write(`${JSON.stringify({ schemas: uris.length })}\n`)
