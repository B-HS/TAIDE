import { PieceTreeTextBufferBuilder } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/pieceTreeTextBuffer/pieceTreeTextBufferBuilder.js'
import { SearchParams, TextModelSearch } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModelSearch.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'

const LINE_COUNT = 300000
const DOCUMENT_BYTES = 20 * 1024 * 1024
const MATCH_LIMIT = 19999
const INDEX_WIDTH = 6
const lineBytes = Math.floor(DOCUMENT_BYTES / LINE_COUNT)
const longerLines = DOCUMENT_BYTES % LINE_COUNT
const lines = Array.from({ length: LINE_COUNT }, (_, index) => {
    const length = lineBytes + Number(index < longerLines)
    return `entry_${index.toString().padStart(INDEX_WIDTH, '0')} sample alpha beta gamma delta `.padEnd(length - 1, 'a') + '\n'
})
const text = lines.join('')
if (text.length !== DOCUMENT_BYTES) {
    throw new Error('Unexpected document length')
}
const builder = new PieceTreeTextBufferBuilder()
builder.acceptChunk(text)
const { textBuffer } = builder.finish().create(1)
const model = {
    getLineCount: () => textBuffer.getLineCount(),
    getLineContent: (line) => textBuffer.getLineContent(line),
    getLineMaxColumn: (line) => textBuffer.getLineContent(line).length + 1,
    getValueInRange: (range, preference) => textBuffer.getValueInRange(range, preference ?? 0),
    getPositionAt: (offset) => textBuffer.getPositionAt(offset),
    getOffsetAt: (position) => textBuffer.getOffsetAt(position.lineNumber, position.column),
    getEOL: () => '\n',
}
const range = new Range(1, 1, model.getLineCount(), model.getLineMaxColumn(model.getLineCount()))
const started = performance.now()
const noMatchCount = TextModelSearch.findMatches(model, new SearchParams('missing_\\d+', true, false, null), range, true, MATCH_LIMIT).length
const noMatchMs = performance.now() - started
const matchingStarted = performance.now()
const limitCount = TextModelSearch.findMatches(model, new SearchParams('entry_\\d+', true, false, null), range, true, MATCH_LIMIT).length
const limitMs = performance.now() - matchingStarted
if (noMatchCount !== 0 || limitCount !== MATCH_LIMIT) {
    throw new Error('Unexpected search result')
}
textBuffer.dispose()
console.info({ bytes: text.length, lines: LINE_COUNT, noMatchCount, noMatchMs, limitCount, limitMs })
process.exit(0)
