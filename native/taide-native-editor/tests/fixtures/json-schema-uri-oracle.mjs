import { URI } from '../../../../node_modules/monaco-editor/esm/external/vscode-uri/lib/esm/index.js'

import { rustLiteral } from './emmet-rust-literal.mjs'

const SOURCE_URL = new URL(
    '../../../../node_modules/monaco-editor/esm/external/vscode-json-languageservice/lib/esm/services/jsonSchemaService.js',
    import.meta.url,
)
const PATCH_URL = new URL('file:///private/tmp/taide-json-schema-uri-source-oracle.mjs')
const source = await Bun.file(SOURCE_URL).text()
const MARKERS = ['function normalizeId(id)', 'function normalizeResourceForMatching(resource)']
for (const marker of MARKERS) {
    if (source.split(marker).length !== 2) throw new Error(`Unexpected source URI marker: ${marker}`)
}
const patched = source.replace(
    /(from\s+|import\s+)'(\.[^']+)'/g,
    (_, prefix, relative) => `${prefix}${JSON.stringify(new URL(relative, SOURCE_URL).href)}`,
)
await Bun.write(PATCH_URL, `${patched}\nexport { normalizeId, normalizeResourceForMatching };\n`)
const actual = await import(PATCH_URL.href)
const SCHEMES = [
    '',
    'file:',
    'http:',
    'https:',
    'HTTP:',
    'urn:',
    'schemaservice:',
    '1foo:',
    '_foo:',
    'a+b.c-d:',
    'a b:',
    'é:',
    ':',
    'foo\n:',
    'foo\r:',
    'foo\r\n:',
    'foo\u2028:',
    'foo\u2029:',
]
const PATHS = [
    '',
    '/',
    'schema.json',
    '/C:/schema.json',
    'C:/schema.json',
    '//SERVER/C:/schema.json',
    '//User:Pass@EXAMPLE.COM:80/a',
    '//A@B@HOST/a',
    '//[ABCD::EF]:80/a',
    '//İ.ΟΣ.𐐀/漢字',
    '//ΟΣ\ud800ΟΣ/%23',
    '//User%23:P%3F@HOST:80%23%3F/a',
    '//HOST%23%3F/a',
    '/a b/%23%3f/%2f/%25',
    '/%41%FF%42',
    '/%C3%A9/%F0%90%90%80',
    '/%C3%41/%GG%41',
    '/%2F%2F',
    '/a/.././b',
    '/a\\b',
    '/\ud800x\udfff',
    '/line\nend',
]
const SUFFIXES = ['', '#', '?', '?a=%23%3F#part', '#a?b#c', '#%41%FF%42', '?q=漢字#𐐀', '#a\nb', '#a\rb', '#a\u2028b', '#a\u2029b']
const PERCENT_PARTS = ['%41', '%42', '%C3', '%A9', '%F0', '%90', '%80', '%FF', '%00', '%GG', '%G1', '%2F', '%23', '%3F']
const inputs = new Set(SCHEMES.flatMap((scheme) => PATHS.flatMap((path) => SUFFIXES.map((suffix) => `${scheme}${path}${suffix}`))))
for (const one of PERCENT_PARTS) {
    for (const two of PERCENT_PARTS) {
        for (const three of PERCENT_PARTS) inputs.add(`file://HOST/${one}${two}${three}?${one}${two}#${two}${three}`)
    }
}
const units = (value) => rustLiteral(Array.from({ length: value.length }, (_, index) => value.charCodeAt(index).toString(16)).join(','))
const cases = Array.from(inputs, (input) => {
    const id = actual.normalizeId(input)
    const matching = actual.normalizeResourceForMatching(input)
    let normalized
    try {
        normalized = URI.parse(input).toString(true)
    } catch {
        normalized = input
    }
    if (id !== normalized) throw new Error('Actual schema URI adapter differs from installed URI')
    return `    (${units(input)}, ${units(id)}, ${units(matching)}),`
})
await Bun.write(new URL('./json-schema-uri-oracle.rs', import.meta.url), `const CASES: &[(&str, &str, &str)] = &[\n${cases.join('\n')}\n];\n`)
process.stdout.write(`${JSON.stringify({ cases: cases.length, corrections: 0, source: rustLiteral(SOURCE_URL.pathname) })}\n`)
