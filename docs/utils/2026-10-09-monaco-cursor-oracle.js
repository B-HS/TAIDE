import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { ResolvedLanguageConfiguration } from '../../node_modules/monaco-editor/esm/vs/editor/common/languages/languageConfigurationRegistry.js'
import { PieceTreeTextBufferBuilder } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js'
import { BracketPairsTextModelPart } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/bracketPairsTextModelPart/bracketPairsImpl.js'
import { TextModelSearch, SearchParams } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModelSearch.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { Selection } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/selection.js'
import { Position } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/position.js'
import { LineTokens } from '../../node_modules/monaco-editor/esm/vs/editor/common/tokens/lineTokens.js'
import { WordSelectionRangeProvider } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/smartSelect/browser/wordSelections.js'
import { BracketSelectionRangeProvider } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/smartSelect/browser/bracketSelections.js'
import { DEFAULT_WORD_REGEXP, getWordAtText } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/wordHelper.js'
import { isNonEmptyArray } from '../../node_modules/monaco-editor/esm/vs/base/common/arrays.js'

const ROOT = resolve(import.meta.dir, '../..')
const CONTRIBUTIONS = resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib')
const TAB_SIZE = 4
const MATCH_LIMIT = 10000
const WORD_SEPARATORS = '`~!@#$%^&*()-=+[{]}\\|;:\'",.<>/?'
const smartSource = readFileSync(resolve(CONTRIBUTIONS, 'smartSelect/browser/smartSelect.js'), 'utf8')
const selectionRanges = eval(
    `(async () => { ${smartSource.slice(smartSource.indexOf('async function provideSelectionRanges'), smartSource.indexOf("CommandsRegistry.registerCommand('_executeSelectionRangeProvider'"))}; return provideSelectionRanges })()`,
)
const provideRanges = await selectionRanges
const bracketSource = readFileSync(resolve(CONTRIBUTIONS, 'bracketMatching/browser/bracketMatching.js'), 'utf8')
const BracketController = eval(
    `(class { ${bracketSource.slice(bracketSource.indexOf('    jumpToBracket()'), bracketSource.indexOf('    removeBrackets(editSource)'))} })`,
)
const onUnexpectedExternalError = (error) => {
    throw error
}
const revived = JSON.parse(readFileSync(resolve(ROOT, 'native/taide-native-syntax/language-configurations/monaco.json'), 'utf8'), (_, value) =>
    value?.source !== undefined && value?.flags !== undefined ? new RegExp(value.source, value.flags) : value,
)
const texts = [
    'x(a[b]c)y',
    'if (a) {\n    fooBar(1)\n}',
    'before\nBEGIN x END\nafter',
    '(a] b)',
    'a <b> c',
    '  fooBar  \r\nnext',
    '값(\u{1f600}) test',
    'before (x',
    'a] b(c)',
]
let results = []
for (const { languageId, configuration } of revived.languages.filter((row) => row.configuration !== null)) {
    const language = new ResolvedLanguageConfiguration(languageId, configuration)
    const codec = { encodeLanguageId: () => 1, decodeLanguageId: () => languageId }
    for (const text of texts) {
        const lines = text.split(/\r\n|\r|\n/)
        const builder = new PieceTreeTextBufferBuilder()
        builder.acceptChunk(text)
        const { textBuffer } = builder.finish().create(1)
        const model = {
            getLineCount: () => lines.length,
            getLineContent: (line) => lines[line - 1],
            getLineLength: (line) => lines[line - 1].length,
            getLineMaxColumn: (line) => lines[line - 1].length + 1,
            getLineMinColumn: () => 1,
            getLineFirstNonWhitespaceColumn: (line) => lines[line - 1].search(/[^ \t]/) + 1,
            getLineLastNonWhitespaceColumn: (line) => lines[line - 1].search(/[ \t]*$/) + 1,
            getLanguageId: () => languageId,
            getValueLength: () => text.length,
            getValue: () => text,
            getEOL: () => (text.includes('\r\n') ? '\r\n' : '\n'),
            getOptions: () => ({ tabSize: TAB_SIZE }),
            getFullModelRange: () => new Range(1, 1, lines.length, lines.at(-1).length + 1),
            getPositionAt: (offset) => textBuffer.getPositionAt(offset),
            getOffsetAt: (position) => textBuffer.getOffsetAt(position.lineNumber, position.column),
            getValueInRange: (range, preference) => textBuffer.getValueInRange(range, preference ?? 0),
            validatePosition: (position) =>
                new Position(position.lineNumber, Math.max(1, Math.min(position.column, lines[position.lineNumber - 1].length + 1))),
            getWordAtPosition: (position) =>
                getWordAtText(position.column, language.getWordDefinition() ?? DEFAULT_WORD_REGEXP, lines[position.lineNumber - 1], 0),
            tokenization: {
                hasTokens: false,
                backgroundTokenizationState: 2,
                getLineTokens: (line) => LineTokens.createEmpty(lines[line - 1], codec),
            },
        }
        const brackets = new BracketPairsTextModelPart(model, { getLanguageConfiguration: () => language })
        model.bracketPairs = brackets
        const asBytes = (position) => new TextEncoder().encode(text.slice(0, model.getOffsetAt(position))).length
        const pairBytes = (pair) => pair?.map((range) => [asBytes(range.getStartPosition()), asBytes(range.getEndPosition())]) ?? null
        let offset = 0
        for (const character of [...text, '']) {
            const position = model.getPositionAt(offset)
            const matched = brackets.matchBracket(position)
            const enclosing = brackets.findEnclosingBrackets(position)
            const next = brackets.findNextBracket(position)
            const ranges = (
                await provideRanges({ all: () => [] }, model, [position], { selectSubwords: true, selectLeadingAndTrailingWhitespace: true }, {})
            )[0]
            let selections = [Selection.fromPositions(position, position)]
            const editor = {
                hasModel: () => true,
                getModel: () => model,
                getSelections: () => selections,
                setSelections: (next) => {
                    selections = next
                },
                revealRange: () => {},
            }
            BracketController.prototype.jumpToBracket.call({ _editor: editor })
            const jump = asBytes(selections[0].getPosition())
            selections = [Selection.fromPositions(position, position)]
            BracketController.prototype.selectToBracket.call({ _editor: editor }, true)
            const select = selections.map((selection) => [asBytes(selection.getSelectionStart()), asBytes(selection.getPosition())])
            results = [
                ...results,
                {
                    language: languageId,
                    text,
                    byte: asBytes(position),
                    matched: pairBytes(matched),
                    enclosing: pairBytes(enclosing),
                    next: next ? pairBytes([next.range])[0] : null,
                    jump,
                    select,
                    ranges: ranges.map((range) => [asBytes(range.getStartPosition()), asBytes(range.getEndPosition())]),
                },
            ]
            offset += character.length
        }
        brackets.dispose()
        textBuffer.dispose()
    }
}
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/cursor-ranges-reference.json'), `${JSON.stringify(results)}\n`)
const source = readFileSync(resolve(CONTRIBUTIONS, 'multicursor/browser/multicursor.js'), 'utf8')
const MultiCursorSession = eval(
    `(() => { ${source.slice(source.indexOf('class MultiCursorSessionResult'), source.indexOf('class MultiCursorSelectionController'))}; return MultiCursorSession })()`,
)
let matches = []
for (const text of ['cat concatenate cat CAT cat', 'a\r\nb a\r\nb', 'i I ı İ ſ S σ ς Σ ß ẞ K k \u{1f600}\u{1f600}']) {
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
    model.findMatches = (needle, _, regex, matchCase, separators) =>
        TextModelSearch.findMatches(
            model,
            new SearchParams(needle, regex, matchCase, separators),
            new Range(1, 1, lines.length, lines.at(-1).length + 1),
            false,
            MATCH_LIMIT,
        )
    for (const needle of ['cat', 'a\nb', 'i', 's', 'σ', 'ß', 'k', '\u{1f600}']) {
        for (const matchCase of [false, true]) {
            const editor = { hasModel: () => true, getModel: () => model, getOption: () => WORD_SEPARATORS }
            const session = new MultiCursorSession(editor, { highlightFindOptions: () => {} }, false, needle, false, matchCase, null)
            const found = session
                .selectAll()
                .map(({ range }) => [
                    new TextEncoder().encode(text.slice(0, model.getOffsetAt(range.getStartPosition()))).length,
                    new TextEncoder().encode(text.slice(0, model.getOffsetAt(range.getEndPosition()))).length,
                ])
            matches = [...matches, { text, needle, match_case: matchCase, ranges: found }]
        }
    }
    textBuffer.dispose()
}
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/cursor-matches-reference.json'), `${JSON.stringify(matches)}\n`)
process.exit(0)
