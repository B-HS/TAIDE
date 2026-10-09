import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { Selection } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/selection.js'
import {
    ReplaceCommand,
    ReplaceCommandThatPreservesSelection,
} from '../../node_modules/monaco-editor/esm/vs/editor/common/commands/replaceCommand.js'
import { PieceTreeTextBufferBuilder } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js'
import { TextModel } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModel.js'
import { IntervalNode, nodeAcceptEdit } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/intervalTree.js'

const ROOT = resolve(import.meta.dir, '../..')
const HIGH_SURROGATE_START = 0xd800
const HIGH_SURROGATE_END = 0xdbff
const GROWS_BEFORE = 2
const GROWS_AFTER = 3
const source = readFileSync(resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations/browser/linesOperations.js'), 'utf8')
const action = eval(
    `class EditorAction {}\n${source.slice(source.indexOf('class TransposeAction'), source.indexOf('class AbstractCaseAction'))}\nTransposeAction`,
)
const inputs = [
    '',
    'a',
    'abcd',
    'a\nb',
    '\nb',
    'ab\n',
    'a\r\nb',
    '\r\nb',
    'a\u{1f600}b',
    '\u{1f600}x',
    'x\u{1f600}',
    '\u{1f600}\nq',
    'e\u{301}x',
    '\u{1f600}\u{1f4a1}x',
]
let cases = []
for (const text of inputs) {
    for (let offset = 0; offset <= text.length; offset += 1) {
        const previous = text.charCodeAt(offset - 1)
        if (previous >= HIGH_SURROGATE_START && previous <= HIGH_SURROGATE_END) continue
        if (text[offset - 1] === '\r' && text[offset] === '\n') continue
        const builder = new PieceTreeTextBufferBuilder()
        builder.acceptChunk(text)
        const { textBuffer } = builder.finish().create(1)
        const model = {
            _buffer: textBuffer,
            _assertNotDisposed: () => {},
            getLineCount: () => textBuffer.getLineCount(),
            getLineMaxColumn: (line) => textBuffer.getLineLength(line) + 1,
            _isValidRange: TextModel.prototype._isValidRange,
            _isValidPosition: TextModel.prototype._isValidPosition,
            _validatePosition: TextModel.prototype._validatePosition,
            validateRange: TextModel.prototype.validateRange,
            getValueInRange: TextModel.prototype.getValueInRange,
        }
        const position = textBuffer.getPositionAt(offset)
        const selected = Selection.fromPositions(position)
        let commands = []
        action.prototype.run.call({}, null, {
            getSelections: () => [selected],
            getModel: () => model,
            pushUndoStop: () => {},
            executeCommands: (_, value) => {
                commands = value
            },
        })
        let after = selected
        for (const command of commands) {
            let edits = []
            let tracked = null
            command.getEditOperations(model, {
                addTrackedEditOperation: (range, replacement) => {
                    edits = [...edits, { range: model.validateRange(range), text: Buffer.from(replacement).toString(), forceMoveMarkers: false }]
                },
                trackSelection: (selection) => {
                    const trackedOffset = textBuffer.getOffsetAt(selection.startLineNumber, selection.startColumn)
                    tracked = new IntervalNode('selection', trackedOffset, trackedOffset)
                    tracked.setOptions({
                        stickiness: selection.startColumn === model.getLineMaxColumn(selection.startLineNumber) ? GROWS_BEFORE : GROWS_AFTER,
                    })
                    return 'selection'
                },
            })
            for (const edit of edits) {
                const start = textBuffer.getOffsetAt(edit.range.startLineNumber, edit.range.startColumn)
                const end = textBuffer.getOffsetAt(edit.range.endLineNumber, edit.range.endColumn)
                const inserted = edit.text.replace(/\r\n|\r|\n/g, textBuffer.getEOL())
                if (tracked) nodeAcceptEdit(tracked, start, end, inserted.length, false)
            }
            const result = textBuffer.applyEdits(edits, false, true)
            after = command.computeCursorState(model, {
                getInverseEditOperations: () => result.reverseEdits,
                getTrackedSelection: () => Selection.fromPositions(textBuffer.getPositionAt(tracked.start)),
            })
        }
        const expected = Array.from({ length: textBuffer.getLineCount() }, (_, index) => textBuffer.getLineContent(index + 1)).join(
            textBuffer.getEOL(),
        )
        const validated = TextModel.prototype.validatePosition.call(model, after.getPosition())
        const head = textBuffer.getOffsetAt(validated.lineNumber, validated.column)
        cases = [...cases, { text, byte: Buffer.byteLength(text.slice(0, offset)), expected, head: Buffer.byteLength(expected.slice(0, head)) }]
        textBuffer.dispose()
    }
}
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/tests/fixtures/transpose-reference.json'), `${JSON.stringify(cases)}\n`)
process.exit(0)
