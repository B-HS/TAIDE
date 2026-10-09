import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { ResolvedLanguageConfiguration } from '../../node_modules/monaco-editor/esm/vs/editor/common/languages/languageConfigurationRegistry.js'
import { LineTokens } from '../../node_modules/monaco-editor/esm/vs/editor/common/tokens/lineTokens.js'
import { Selection } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/selection.js'
import { MoveLinesCommand } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations/browser/moveLinesCommand.js'
import { ShiftCommand } from '../../node_modules/monaco-editor/esm/vs/editor/common/commands/shiftCommand.js'
import { LineCommentCommand } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/comment/browser/lineCommentCommand.js'
import { BlockCommentCommand } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/comment/browser/blockCommentCommand.js'
import { CopyLinesCommand } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations/browser/copyLinesCommand.js'
import { PieceTreeTextBufferBuilder } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js'
import { TextModel } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModel.js'
import { DEFAULT_WORD_REGEXP, getWordAtText } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/wordHelper.js'

const ROOT = resolve(import.meta.dir, '../..')
const CONFIGURATION = resolve(ROOT, 'native/taide-native-syntax/language-configurations/monaco.json')
const INDENT_SIZE = 4
const FULL_AUTO_INDENT = 4
const LONG_WORD_LENGTH = 1200
const WORD_WINDOW_LENGTH = 1000
const HIGH_SURROGATE_START = 0xd800
const HIGH_SURROGATE_END = 0xdbff
const REMOVE_COMMENT = 2
const revived = JSON.parse(readFileSync(CONFIGURATION, 'utf8'), (_, value) =>
    value?.source !== undefined && value?.flags !== undefined ? new RegExp(value.source, value.flags) : value,
)
let result = []
let words = []
const texts = [
    'before\nif value\n    work\nend\nafter',
    'if value\n    work\nend',
    'before\nif value do\n    work\nend\nafter',
    '/**\n * first\n * second\n */',
    'one\n\n    two\nend',
    'if a\n    if b\n        work\n    end\nend',
    'if a\r\n    work\r\nend',
]
const wordTexts = ['fooBar -1.25px next_word a-b', '한글中文 café λ \u{1f600} test', 'a\u{a0}b 99Bottles', 'x'.repeat(LONG_WORD_LENGTH)]
for (const row of revived.languages.filter((row) => row.configuration !== null)) {
    const { languageId, configuration } = row
    const language = new ResolvedLanguageConfiguration(languageId, configuration)
    const service = { getLanguageConfiguration: () => language }
    const codec = { decodeLanguageId: () => languageId, encodeLanguageId: () => 1 }
    for (const text of texts) {
        const lines = text.split(/\r\n|\r|\n/)
        for (const down of [false, true]) {
            for (let first = 0; first < lines.length; first += 1) {
                for (const count of [1, 2]) {
                    if (first + count > lines.length) continue
                    const selection = new Selection(first + 1, 1, first + count, lines[first + count - 1].length + 1)
                    const model = {
                        getLineCount: () => lines.length,
                        getLineContent: (line) => lines[line - 1],
                        getLineMaxColumn: (line) => lines[line - 1].length + 1,
                        getLineFirstNonWhitespaceColumn: (line) => lines[line - 1].search(/[^ \t]/) + 1,
                        getLanguageIdAtPosition: () => languageId,
                        getLanguageId: () => languageId,
                        getOptions: () => ({ tabSize: INDENT_SIZE, indentSize: INDENT_SIZE, insertSpaces: true }),
                        tokenization: {
                            getLanguageId: () => languageId,
                            getLanguageIdAtPosition: () => languageId,
                            getLineTokens: (line) => LineTokens.createEmpty(lines[line - 1], codec),
                            isCheapToTokenize: () => true,
                            forceTokenization: () => {},
                            tokenizeIfCheap: () => {},
                        },
                    }
                    const commands = {
                        move: () => new MoveLinesCommand(selection, down, FULL_AUTO_INDENT, service),
                        shift: () =>
                            new ShiftCommand(
                                selection,
                                {
                                    tabSize: INDENT_SIZE,
                                    indentSize: INDENT_SIZE,
                                    insertSpaces: true,
                                    useTabStops: true,
                                    isUnshift: down,
                                    autoIndent: FULL_AUTO_INDENT,
                                },
                                service,
                            ),
                        comment: () => new LineCommentCommand(service, selection, INDENT_SIZE, 0, true, true, false),
                        addComment: () => new LineCommentCommand(service, selection, INDENT_SIZE, 1, true, true, false),
                        removeComment: () => new LineCommentCommand(service, selection, INDENT_SIZE, REMOVE_COMMENT, true, true, false),
                        blockComment: () => new BlockCommentCommand(selection, true, service),
                        copy: () => new CopyLinesCommand(selection, down, false),
                    }
                    for (const [command, create] of Object.entries(commands)) {
                        if (!['move', 'shift', 'copy'].includes(command) && down) continue
                        const instance = create()
                        let edits = []
                        const add = (range, text) => {
                            edits = [...edits, { range, text, forceMoveMarkers: false }]
                        }
                        instance.getEditOperations(model, { addEditOperation: add, addTrackedEditOperation: add, trackSelection: () => 'selection' })
                        const builder = new PieceTreeTextBufferBuilder()
                        builder.acceptChunk(text)
                        const { textBuffer } = builder.finish().create(1)
                        const validation = {
                            _buffer: textBuffer,
                            _assertNotDisposed: () => {},
                            getLineCount: () => textBuffer.getLineCount(),
                            getLineMaxColumn: (line) => textBuffer.getLineLength(line) + 1,
                            _isValidRange: TextModel.prototype._isValidRange,
                            _isValidPosition: TextModel.prototype._isValidPosition,
                            _validatePosition: TextModel.prototype._validatePosition,
                        }
                        const validated = edits.map((edit) => ({ ...edit, range: TextModel.prototype.validateRange.call(validation, edit.range) }))
                        textBuffer.applyEdits(validated, false, false)
                        const expected = Array.from({ length: textBuffer.getLineCount() }, (_, index) => textBuffer.getLineContent(index + 1)).join(
                            textBuffer.getEOL(),
                        )
                        result = [...result, { languageId, command, down, text, first, count, expected }]
                        textBuffer.dispose()
                    }
                }
            }
        }
    }
    for (const text of wordTexts) {
        const offsets =
            text.length > WORD_WINDOW_LENGTH
                ? [0, 1, 499, 500, 501, 599, 600, 601, text.length - 1, text.length]
                : Array.from({ length: text.length + 1 }, (_, index) => index)
        for (const byte of offsets) {
            const previous = text.charCodeAt(byte - 1)
            if (previous >= HIGH_SURROGATE_START && previous <= HIGH_SURROGATE_END) continue
            const word = getWordAtText(byte + 1, configuration.wordPattern ?? DEFAULT_WORD_REGEXP, text, 0)
            words = [...words, { languageId, text, offset: byte, range: word ? [word.startColumn - 1, word.endColumn - 1] : null }]
        }
    }
}
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/line-commands-reference.json'), `${JSON.stringify(result)}\n`)
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/word-ranges-reference.json'), `${JSON.stringify(words)}\n`)
process.exit(0)
