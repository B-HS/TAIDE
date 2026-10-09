import { readFileSync, writeFileSync } from 'node:fs'

import { URI } from '../../node_modules/monaco-editor/esm/vs/base/common/uri.js'
import { DEFAULT_WORD_REGEXP, getWordAtText } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/wordHelper.js'
import { TextModel } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/textModel.js'
import { MarkerDecorationsService } from '../../node_modules/monaco-editor/esm/vs/editor/common/services/markerDecorationsService.js'

const VERSION = '0.56.0'
const OUTPUT = new URL('../../native/taide-native-editor/tests/fixtures/diagnostic-reference.txt', import.meta.url)
const CORPUS = ['abc def', 'a\u{1f600}한\n\nend', '', '\n', '\t abc  ', 'first\r\nsecond\r\n']
const OUTSIDE = 99
const SEVERITIES = [8, 4, 2, 1]
const noop = () => ({ dispose: () => {} })
const version = JSON.parse(readFileSync(new URL('../../node_modules/monaco-editor/package.json', import.meta.url), 'utf8')).version
if (version !== VERSION) throw new Error('Monaco diagnostic reference version changed')

const reference = (text) => {
    const lines = text.split(/\r?\n/)
    const model = Object.assign(Object.create(TextModel.prototype), {
        _isDisposed: false,
        _buffer: {
            getLineCount: () => lines.length,
            getLineLength: (line) => lines[line - 1].length,
            getLineContent: (line) => lines[line - 1],
            getLineCharCode: (line, column) => lines[line - 1].charCodeAt(column),
            getLineLastNonWhitespaceColumn: (line) => lines[line - 1].trimEnd().length + 1,
            getLineFirstNonWhitespaceColumn: (line) => lines[line - 1].search(/\S/) + 1,
        },
        _associatedResource: URI.parse('file:///synthetic/reference.txt'),
        getWordAtPosition: (position) => getWordAtText(position.column, DEFAULT_WORD_REGEXP, lines[position.lineNumber - 1], 0),
        deltaDecorations: () => [],
    })
    const service = new MarkerDecorationsService(
        { getModels: () => [model], onModelAdded: noop, onModelRemoved: noop },
        { read: () => [], onMarkerChanged: noop },
    )
    const decorations = service._markerDecorations.get(model.uri)
    const coordinates = lines.flatMap((line, index) => [0, 1, 2, line.length, OUTSIDE].map((column) => [index, column]))
    const ranges = coordinates.flatMap(([line, column]) => [
        [line, column, line, column],
        [line, column, line, column + 1],
        [line, column, lines.length + 1, OUTSIDE],
    ])
    ranges.push([OUTSIDE, 0, OUTSIDE + 1, 0])
    const byteAt = (line, column) => {
        const prefix = text
            .split('\n')
            .slice(0, line - 1)
            .map((line) => `${line}\n`)
            .join('')
        return Buffer.byteLength(prefix) + Buffer.byteLength(lines[line - 1].slice(0, column - 1))
    }
    const cases = ranges.flatMap(([startLine, startColumn, endLine, endColumn]) =>
        SEVERITIES.map((severity) => {
            const raw = {
                startLineNumber: startLine + 1,
                startColumn: startColumn + 1,
                endLineNumber: endLine + 1,
                endColumn: endColumn + 1,
                severity,
            }
            const range = decorations._createDecorationRange(model, raw)
            const options = decorations._createDecorationOption(raw)
            return {
                text,
                severity,
                range: [startLine, startColumn, endLine, endColumn],
                bytes: [byteAt(range.startLineNumber, range.startColumn), byteAt(range.endLineNumber, range.endColumn)],
                className: options.className,
            }
        }),
    )
    service.dispose()
    return cases
}

const cases = CORPUS.flatMap(reference)
writeFileSync(
    OUTPUT,
    `${VERSION}\n${cases.map((entry) => [Buffer.from(entry.text).toString('hex'), entry.severity, ...entry.range, ...entry.bytes, entry.className].join('\t')).join('\n')}\n`,
)
console.info(JSON.stringify({ version: VERSION, cases: cases.length }))
process.exit(0)
