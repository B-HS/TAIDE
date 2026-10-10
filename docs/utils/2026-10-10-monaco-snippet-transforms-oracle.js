import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { SnippetParser, Variable } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetParser.js'

const ROOT = resolve(import.meta.dir, '../..')
const VALUES = [
    '',
    'hello world',
    'helloWorld',
    'XMLHttpRequest2',
    'URL42 __Http_server--name',
    '___LOUD_name___',
    'FOO2Bar3 abcXYZ',
    '  foo\tbar\nBaz ',
    'ßtraße',
    'İSTANBUL ΣΟΣ',
    'αΒΓδ_ω',
    '한글 中文',
    '𐐀word 𐐨word',
    '\u{1f600}\u{1f642}abc',
    '... - _',
    '\ufeffFoo\u0085Bar\ufeff',
    '123abc ABC123 xYz9',
    'aA aAA aAa',
    'A B ABC ABc ABcDEF',
    'no-match',
]
const SHORTHANDS = ['upcase', 'downcase', 'capitalize', 'pascalcase', 'camelcase', 'snakecase', 'kebabcase', 'unknown']
const FORMATS = ['$0', '$1:$2:$3', '${1:+present}', '${1:-missing}', '${1:?yes:no}', '${1:default}', '${9:?yes:}', '${9:+}', '${1:/unknown}']
const PATTERNS = [
    ['(.*)', ''],
    ['(.*)', 's'],
    ['(.)(.?)', 'g'],
    ['(.)(.?)', 'gu'],
    ['^(.+)$', 'gm'],
    ['a', 'giy'],
    ['a', 'y'],
    ['', 'g'],
    ['', 'gu'],
    ['(z+)', ''],
    ['(a)?(b)?', 'g'],
    ['(?<=a)(b)', 'g'],
    ['(𐐀)', 'gi'],
    ['(𐐀)', 'giu'],
    ['(\\p{L}+)', 'gu'],
    ['(\\p{L}+)', 'gv'],
    ['([\\p{ASCII}&&\\p{Letter}]+)', 'gv'],
]
const evaluate = (pattern, flags, format, value) => {
    const snippet = '${name/' + pattern.replaceAll('/', '\\/') + '/' + format + '/' + flags + '}'
    const parsed = new SnippetParser().parse(snippet, false, false)
    const variable = parsed.children[0]
    if (!(variable instanceof Variable) || !variable.transform) {
        throw new Error('Original Monaco did not parse transform: ' + snippet)
    }
    return { snippet, value, expected: Buffer.from(variable.transform.resolve(value)).toString() }
}
const cases = [
    ...SHORTHANDS.flatMap((shorthand) => VALUES.map((value) => evaluate('(.*)', 's', '${1:/' + shorthand + '}', value))),
    ...PATTERNS.flatMap(([pattern, flags]) => FORMATS.flatMap((format) => VALUES.map((value) => evaluate(pattern, flags, format, value)))),
    ...['', 'g', 'u', 'gu', 'v', 'gv'].flatMap((flags) => ['x', '$0', '$1$1'].map((format) => evaluate('(.)', flags, format, '\u{1f642}'))),
    evaluate('(?<first>a)(?<second>b)', 'g', '$0:$1:$2', 'ab ab'),
    evaluate('a/b', '', '$0', 'a/b'),
]
const sources = ['', 'a/b', 'a\\/b', '\n', '\r', '\u2028', '\u2029', '\\\n', '\u{1f642}', '[\\p{L}&&[^x]]']
const flags = ['', 'i', 'ygim', 's', 'msu', 'd', 'v', 'iug', 'gg', 'ii', 'uuv', 'uv', 'q']
const metadata = sources.flatMap((source) =>
    flags.map((options) => {
        try {
            const regexp = new RegExp(source, options)
            return { source, options, expected: { source: regexp.source, ignore_case: regexp.ignoreCase, global: regexp.global } }
        } catch {
            return { source, options, expected: null }
        }
    }),
)
writeFileSync(
    resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/snippet-transforms-reference.json'),
    JSON.stringify({ cases, metadata }, null, 4) + '\n',
)
process.stdout.write(JSON.stringify({ transforms: cases.length, metadata: metadata.length }) + '\n')
process.exit(0)
