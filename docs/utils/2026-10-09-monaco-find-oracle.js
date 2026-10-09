import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { PieceTreeTextBufferBuilder } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js'
import { SearchParams, TextModelSearch } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModelSearch.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { ReplacePattern, parseReplaceString } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/find/browser/replacePattern.js'

const ROOT = resolve(import.meta.dir, '../..')
const MATCH_LIMIT = 19999
const WORD_SEPARATORS = '`~!@#$%^&*()-=+[{]}\\|;:\'",.<>/?'
const EXPECTED_VERSION = '0.56.0'
const EXPECTED_REF = 'f487add297079a02eb836810185b165e50cadabc'
const encoder = new TextEncoder()
const bytesBefore = (text, offset) => encoder.encode(text.slice(0, offset)).length
const startOffsets = (text) => [...new Set([0, [...text][0]?.length ?? 0, text.length])]
const packageMetadata = JSON.parse(readFileSync(resolve(ROOT, 'node_modules/monaco-editor/package.json'), 'utf8'))
if (packageMetadata.version !== EXPECTED_VERSION || packageMetadata.vscodeRef !== EXPECTED_REF) {
    throw new Error('Monaco reference changed')
}

const engineTexts = [
    '',
    'aab abb aaab',
    'cat concatenate CAT cat',
    '한a 값 가',
    'é 123 １２٣',
    'ſ S K K k Σ σ ς ß ẞ İ ı I i',
    '\u{1f600}\u{1f600} a',
    'a\nb\r\nc\r\nd\u2028e\u2029f',
    'foo_bar Foo-Bar fooBar',
]
const enginePatterns = [
    '',
    '.',
    '.*',
    '^',
    '$',
    '^$',
    '^.$',
    '\\d+',
    '\\d*',
    '\\w+',
    '\\b.',
    '\\B.',
    '\\s+',
    '\\W+',
    '(?<=a)b',
    '(?<!a)b',
    '(?<=a+)b',
    'a(?=b)',
    'a(?!b)',
    '(a)\\1',
    '(?<n>a)\\k<n>',
    '(a)?(b)',
    '(a)|(b)',
    '\\p{L}+',
    '\\p{Script=Hangul}+',
    '\\u{1F600}',
    'ß',
    'Σ',
    'K',
    'i',
    '(',
    '[',
    'a{',
    '\\-',
    '(?<n>a)(?<n>b)',
    '(?>a)',
    '(?i)a',
    '\\p{Script=UnknownScript}',
]
const engineCases = enginePatterns.flatMap((source) =>
    ['u', 'iu', 'mu', 'imu'].flatMap((flags) =>
        engineTexts.flatMap((text) =>
            startOffsets(text).map((offset) => {
                const input = { source, flags, text, start: bytesBefore(text, offset) }
                try {
                    const expression = new RegExp(source, `${flags}gd`)
                    expression.lastIndex = offset
                    const found = expression.exec(text)
                    const captures = found?.indices.map((range) => range?.map((boundary) => bytesBefore(text, boundary)) ?? null) ?? null
                    return { ...input, isError: false, captures }
                } catch {
                    return { ...input, isError: true, captures: null }
                }
            }),
        ),
    ),
)

const modelTexts = [
    'cat concatenate CAT cat',
    'aab abb\naaab\n',
    'aab abb\r\naaab\r\n',
    '한a é １２٣ 123\nſ S K K σ ς Σ ß ẞ',
    '\u{1f600}\u{1f600} a\n\u{1f600}',
    'foo_bar Foo-Bar fooBar',
    '\n\n',
    'cat\u00a0cat\u2028cat\u2029cat\tcat',
]
const modelPatterns = [
    '',
    'cat',
    'a',
    '.',
    '.*',
    '^',
    '$',
    '^$',
    '\\d*',
    '\\w+',
    '\\b.',
    '\\s+',
    '\\n',
    '\\r',
    '\\W+',
    '(?<=a)b',
    '(a)?(b)',
    '\\p{L}+',
    '(',
]
const modelCases = modelTexts.flatMap((text) => {
    const lines = text.split(/\r\n|\r|\n/)
    const builder = new PieceTreeTextBufferBuilder()
    builder.acceptChunk(text)
    const { textBuffer } = builder.finish().create(1)
    const model = {
        getLineCount: () => lines.length,
        getLineContent: (line) => lines[line - 1],
        getLineMaxColumn: (line) => lines[line - 1].length + 1,
        getValueInRange: (range, preference) => textBuffer.getValueInRange(range, preference ?? 0),
        getPositionAt: (offset) => textBuffer.getPositionAt(offset),
        getOffsetAt: (position) => textBuffer.getOffsetAt(position.lineNumber, position.column),
        getEOL: () => (text.includes('\r\n') ? '\r\n' : '\n'),
    }
    const fullRange = new Range(1, 1, lines.length, lines.at(-1).length + 1)
    const firstCharacterEnd = model.getPositionAt([...text][0]?.length ?? 0)
    const innerRange = new Range(firstCharacterEnd.lineNumber, firstCharacterEnd.column, fullRange.endLineNumber, fullRange.endColumn)
    const bytePosition = (position) => bytesBefore(text, model.getOffsetAt(position))
    const matchReference = (found) =>
        found
            ? {
                  range: [bytePosition(found.range.getStartPosition()), bytePosition(found.range.getEndPosition())],
                  captures: found.matches.map((capture) => capture ?? null),
              }
            : null
    const cases = modelPatterns.flatMap((source) =>
        [false, true].flatMap((isRegex) =>
            [false, true].flatMap((matchCase) =>
                [false, true].flatMap((wholeWord) => {
                    const parameters = new SearchParams(source, isRegex, matchCase, wholeWord ? WORD_SEPARATORS : null)
                    const fullMatches = TextModelSearch.findMatches(model, parameters, fullRange, true, MATCH_LIMIT)
                    const navigation = startOffsets(text).map((offset) => {
                        const position = model.getPositionAt(offset)
                        return {
                            start: bytePosition(position),
                            next: matchReference(TextModelSearch.findNextMatch(model, parameters, position, true)),
                            previous: matchReference(TextModelSearch.findPreviousMatch(model, parameters, position, true)),
                            previousWithFullContext: matchReference(
                                fullMatches.filter((found) => bytePosition(found.range.getEndPosition()) <= bytePosition(position)).at(-1) ??
                                    fullMatches.at(-1),
                            ),
                        }
                    })
                    return [fullRange, innerRange].map((range) => ({
                        text,
                        source,
                        isRegex,
                        matchCase,
                        wholeWord,
                        scope: [bytePosition(range.getStartPosition()), bytePosition(range.getEndPosition())],
                        matches: TextModelSearch.findMatches(model, parameters, range, true, MATCH_LIMIT).map(matchReference),
                        navigation,
                    }))
                }),
            ),
        ),
    )
    textBuffer.dispose()
    return cases
})

const replacements = [
    '',
    'text',
    'New-Text',
    'New_Text',
    'MiX-ed_Value',
    '\n',
    '\\n',
    '\\t',
    '\\\\',
    '\\r',
    '\\',
    '$',
    '$$',
    '$&',
    '$0',
    '$01',
    '$1',
    '$2',
    '$3',
    '$9',
    '$10',
    '$12',
    '$99',
    '$100',
    '$<name>',
    '$`',
    "$'",
    '$1$2',
    '\\u$1',
    '\\U$1',
    '\\l$1',
    '\\L$1',
    '\\u\\L$1',
    '\\u\\u$1',
    '\\uX$1',
    '\\u$1$2',
    '\\u\\\\$1',
    '\\$1',
    '$\\n',
    '\\U$9',
    '\\L$&',
]
const captureSets = [
    ['', ''],
    ['abc', 'abc'],
    ['ABC', 'ABC'],
    ['aBc', 'aBc'],
    ['foo-bar', 'foo', 'bar'],
    ['FOO-bar', 'FOO', 'bar'],
    ['Foo_bar', 'Foo', 'bar'],
    ['FOO-BAR', 'FOO', 'BAR'],
    ['1', '1'],
    ['Σσ', 'Σσ'],
    ['ΟΣ', 'ΟΣ'],
    ['İß', 'İß'],
    ['\u{1f600}Bob', '\u{1f600}Bob'],
    ['\u{10428} abc', '\u{10428} abc'],
    ['b', undefined, 'b'],
    ['Aa_bB-Cc', 'Aa_bB-Cc'],
]
const replacementCases = replacements.flatMap((input) =>
    [false, true].flatMap((isRegex) =>
        [false, true].flatMap((preserveCase) =>
            captureSets.map((captures) => {
                const pattern = isRegex ? parseReplaceString(input) : ReplacePattern.fromStaticValue(input)
                return {
                    input,
                    isRegex,
                    preserveCase,
                    captures: captures.map((capture) => capture ?? null),
                    expected: pattern.buildReplaceString(captures, preserveCase),
                    hasCaptures: pattern.hasReplacementPatterns,
                }
            }),
        ),
    ),
)
const reference = { version: EXPECTED_VERSION, vscodeRef: EXPECTED_REF, engineCases, modelCases, replacementCases }
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/find-reference.json'), `${JSON.stringify(reference)}\n`)
console.info({ engineCases: engineCases.length, modelCases: modelCases.length, replacementCases: replacementCases.length })
process.exit(0)
