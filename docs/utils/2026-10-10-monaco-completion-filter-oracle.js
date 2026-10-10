import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { anyScore, fuzzyScore, fuzzyScoreGracefulAggressive } from '../../node_modules/monaco-editor/esm/vs/base/common/filters.js'

const ROOT = resolve(import.meta.dir, '../..')
const FILTER_LIMIT = 128
const LONG_PREFIX_LENGTH = FILTER_LIMIT - 1
const CONFIGURATIONS = [
    { firstMatchCanBeWeak: false, boostFullMatch: true },
    { firstMatchCanBeWeak: true, boostFullMatch: true },
    { firstMatchCanBeWeak: false, boostFullMatch: false },
    { firstMatchCanBeWeak: true, boostFullMatch: false },
]
const WORDS = [
    '',
    'console',
    'Console',
    'cOnSoLe',
    'fooBarBaz',
    'foobarfoobar',
    'bar_foo',
    'FooBar',
    '.foo.bar',
    '  foo',
    '\tfoo',
    '$console',
    'URLParser',
    'foo/Foo',
    'foo\\bar',
    '한글Console',
    '\u{1f600}foo',
    'foo\u{1f600}bar',
    'İstanbul',
    'Σίσυφος',
    'ΟΣ',
    'e\u0301Foo',
    'Straße',
    'foo-bar(foo)',
    'const console =',
    `${'a'.repeat(LONG_PREFIX_LENGTH)}b`,
    `${'a'.repeat(FILTER_LIMIT)}b`,
]
const PATTERNS = [
    '',
    'c',
    'con',
    'cno',
    'cnoso',
    'consle',
    'console',
    'Console',
    'cl',
    'fb',
    'fbb',
    'FooB',
    'bar',
    'foo',
    '.f',
    '  f',
    '\tf',
    '한글',
    '\u{1f600}f',
    'İs',
    'σ',
    'ος',
    'e\u0301',
    `${'a'.repeat(LONG_PREFIX_LENGTH)}b`,
    `${'a'.repeat(FILTER_LIMIT)}b`,
]
const METHODS = {
    normal: (pattern, start, word, wordStart, options) =>
        fuzzyScore(pattern, pattern.toLowerCase(), start, word, word.toLowerCase(), wordStart, options),
    graceful: (pattern, start, word, wordStart, options) =>
        fuzzyScoreGracefulAggressive(pattern, pattern.toLowerCase(), start, word, word.toLowerCase(), wordStart, options),
    any: (pattern, start, word, wordStart) => anyScore(pattern, pattern.toLowerCase(), start, word, word.toLowerCase(), wordStart),
}
const rows = WORDS.flatMap((word, wordIndex) =>
    PATTERNS.flatMap((pattern, patternIndex) =>
        CONFIGURATIONS.flatMap((options, optionIndex) =>
            Object.entries(METHODS).map(([method, score]) => {
                const patternStart = optionIndex % 2
                const wordStart = (wordIndex + patternIndex) % 2
                const result = score(pattern, patternStart, word, wordStart, options)
                const positions = result?.slice(2).join(',') ?? ''
                return [
                    method,
                    Buffer.from(pattern).toString('hex'),
                    Buffer.from(word).toString('hex'),
                    patternStart,
                    wordStart,
                    Number(options.firstMatchCanBeWeak),
                    Number(options.boostFullMatch),
                    result?.[0] ?? 'none',
                    result?.[1] ?? wordStart,
                    positions.length > 0 ? positions : 'none',
                ].join('\t')
            }),
        ),
    ),
)
writeFileSync(resolve(ROOT, 'native/taide-native-editor/tests/fixtures/completion-filter-reference.tsv'), `${rows.join('\n')}\n`)
process.stdout.write(`Monaco completion filter: ${rows.length} samples\n`)
