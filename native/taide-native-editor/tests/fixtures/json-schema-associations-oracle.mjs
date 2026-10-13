import { createRegex } from '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/utils/glob.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const PATCH_URL = new URL('file:///private/tmp/taide-json-schema-associations-source-oracle.mjs')
const source = await Bun.file(SOURCE_URL).text()
const MARKERS = ['class FilePatternAssociation', 'function normalizeId(id)', 'function normalizeResourceForMatching(resource)']
for (const marker of MARKERS) {
    if (source.split(marker).length !== 2) throw new Error(`Unexpected association source marker: ${marker}`)
}
const patched = source.replace(
    /(from\s+|import\s+)'(\.[^']+)'/g,
    (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`,
)
await Bun.write(PATCH_URL, `${patched}\nexport { FilePatternAssociation, normalizeId, normalizeResourceForMatching };\n`)
const actual = await import(PATCH_URL.href)
const GLOB_URL = new URL('../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/utils/glob.js', import.meta.url)
const GLOB_PATCH_URL = new URL('file:///private/tmp/taide-json-schema-glob-source-oracle.mjs')
const glob = await Bun.file(GLOB_URL).text()
const GLOB_MARKER = 'return new RegExp(reStr, flags);'
if (glob.split(GLOB_MARKER).length !== 2) throw new Error('Unexpected original glob constructor marker')
await Bun.write(GLOB_PATCH_URL, glob.replace(GLOB_MARKER, 'return { source: reStr, flags };'))
const originalBuilder = await import(GLOB_PATCH_URL.href)
const PATTERNS = [
    [],
    [''],
    ['!'],
    ['/'],
    ['*'],
    ['*.json'],
    ['**/*.json'],
    ['**'],
    ['configs/*.json'],
    ['/configs/*.json'],
    ['?file.json'],
    ['??file.json'],
    ['[ab].json'],
    ['[^ab].json'],
    ['[!a].json'],
    ['[z-a].json'],
    ['['],
    [']'],
    ['{a,b}.json'],
    ['{a,{b,c}}.json'],
    ['a,b.json'],
    ['{**/a,**/b}.json'],
    ['a/**/b.json'],
    ['a/**'],
    ['a**b.json'],
    ['***/*.json'],
    ['*.json', '!a.json'],
    ['!a.json', '*.json'],
    ['*.json', '!*.json', 'a.json'],
    ['*.json', '['],
    ['*.json', '!'],
    ['?.json'],
    ['??.json'],
    ['𐐀.json'],
    ['\ud800.json'],
    ['漢字.json'],
    ['a\\b.json'],
    ['a\\.json'],
    ['*.{json,yaml}'],
    ['!**/private/**', '**/*.json', '!**/secret.json'],
    ['a\n.json'],
    ['a\u2028.json'],
    ['$.json'],
    ['a+b(c)=d!|e.json'],
    ['{*.json,*.yaml}'],
    ['**/{a,b}/**'],
]
const FOLDERS = [undefined, '', 'file:///workspace', 'file:///WORKSPACE/', 'file:///C:/workspace?x#y', 'file://HOST/workspace', 'a b:/workspace']
const RESOURCES = [
    'file:///workspace/a.json',
    'file:///workspace/b.json',
    'file:///workspace/c.json',
    'file:///workspace/x.yaml',
    'file:///workspace/a/b.json',
    'file:///workspace/configs/a.json',
    'file:///workspace/private/a.json',
    'file:///workspace/a/secret.json',
    'file:///workspace/𐐀.json',
    'file:///workspace/\ud800.json',
    'file:///workspace/漢字.json',
    'file:///workspace/afile.json',
    'file:///workspace/abfile.json',
    'file:///workspace/a\n.json',
    'file:///workspace/a\u2028.json',
    'file:///other/a.json',
    'file:///workspace2/a.json',
    'file:///workspace/a.json?q#f',
    'file:///C:/workspace/a.json',
    'file://HOST/workspace/a.json',
    'file:///WORKSPACE/a.json',
    'file:///workspace/$.json',
    'file:///workspace/a+b(c)=d!|e.json',
    'file:///workspace/a\\b.json',
]
const URIS = ['urn:first#', 'HTTP://HOST/schema#']
const units = (value) => rustLiteral(Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(','))
const list = (values) => `&[${values.map(units).join(',')}]`
const associationCases = []
for (const patterns of PATTERNS) {
    for (const folder of FOLDERS) {
        const uris = URIS.map(actual.normalizeId)
        const association = new actual.FilePatternAssociation(patterns, folder, uris)
        for (const resource of RESOURCES) {
            const matched = association.matchesPattern(actual.normalizeResourceForMatching(resource))
            associationCases.push(
                `    (${list(patterns)}, ${folder == null ? 'None' : `Some(${units(folder)})`}, ${units(resource)}, ${matched}, ${list(association.getURIs())}),`,
            )
        }
    }
}
const globCases = []
for (const input of new Set(PATTERNS.flatMap((patterns) => patterns))) {
    const built = originalBuilder.createRegex(input, { extended: true, globstar: true })
    let is_valid = true
    try {
        createRegex(input, { extended: true, globstar: true })
    } catch {
        is_valid = false
    }
    globCases.push(`    (${units(input)}, ${units(built.source)}, ${is_valid}),`)
}
await Bun.write(
    new URL('./json-schema-associations-oracle.rs', import.meta.url),
    `const ASSOCIATION_CASES: &[(&[&str], Option<&str>, &str, bool, &[&str])] = &[\n${associationCases.join('\n')}\n];\nconst GLOB_CASES: &[(&str, &str, bool)] = &[\n${globCases.join('\n')}\n];\n`,
)
process.stdout.write(`${JSON.stringify({ associations: associationCases.length, globSources: globCases.length, corrections: 0 })}\n`)
