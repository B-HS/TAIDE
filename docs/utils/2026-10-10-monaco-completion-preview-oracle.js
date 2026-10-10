import { readFileSync, writeFileSync } from 'node:fs'
import { SnippetSession } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetSession.js'
import { SnippetParser } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetParser.js'
import { normalizeIndentation } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/misc/indentation.js'
import { commonPrefixLength } from '../../node_modules/monaco-editor/esm/vs/base/common/strings.js'
import { resolve } from 'node:path'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { Position } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/position.js'
import { TextReplacement } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/edits/textEdit.js'
import { computeGhostText } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/inlineCompletions/browser/model/computeGhostText.js'
import { LineDecoration } from '../../node_modules/monaco-editor/esm/vs/editor/common/viewLayout/lineDecorations.js'
import { ColumnRange } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/ranges/columnRange.js'
import { RangeSingleLine } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/ranges/rangeSingleLine.js'

const ROOT = resolve(import.meta.dir, '../..')
const GENERATION_DEPTH = 4
const DIFF_LIMIT = 5000
const MODES = ['prefix', 'subword', 'subwordSmart']
const HALF = 2
const words = (depth) => (depth === 0 ? [''] : ['', ...['a', 'b'].flatMap((letter) => words(depth - 1).map((suffix) => letter + suffix))])
const binary = [...new Set(words(GENERATION_DEPTH))]
const samples = ['', 'foo', 'fooBar', 'if ()', 'if (f())', '(', ')', '()', 'a() b()', ' 한글)', '\tword', '    word', '\u{1f600}a', 'X\u{10400}y']
const substitutions = [
    '',
    'foobar',
    'fooBar()',
    'if (f() = 1) { g(); }',
    '(foo)',
    'a(b()) c()',
    '한글\n    다음)',
    '\t\tword',
    '  word',
    '\u{1f600}abc',
    'X\u{10400}xy',
    'foo\r\nbar',
    'foo\rbar',
]
const basic = [
    ...binary.flatMap((source) => binary.map((text) => ({ source, text }))),
    ...samples.flatMap((source) => substitutions.map((text) => ({ source, text }))),
]
const cases = basic.flatMap(({ source, text }) =>
    [...new Set([0, Math.floor(source.length / HALF), source.length])].flatMap((cursor) =>
        MODES.map((mode) => ({ source, text, cursor, start: 0, end: source.length, mode, suffix: 0 })),
    ),
)
const special = [
    { source: 'prefix ab suffix', start: 7, end: 9, cursor: 8, text: 'aXbYb' },
    { source: '  ab suffix', start: 0, end: 4, cursor: 2, text: '    aXb' },
    { source: '  ab suffix', start: 1, end: 4, cursor: 2, text: '\taXb' },
    { source: 'first\nabc', start: 0, end: 9, cursor: 7, text: 'first\naXbc' },
    { source: 'first\r\nabc', start: 0, end: 10, cursor: 8, text: 'first\r\naXbc' },
    { source: 'first\nabc', start: 0, end: 9, cursor: 7, text: 'other\naXbc' },
    { source: 'foo', start: 0, end: 3, cursor: 3, text: 'fooabcd', suffix: 2 },
    { source: 'a', start: 0, end: 1, cursor: 0, text: 'b' + 'a'.repeat(DIFF_LIMIT - HALF) + 'b' },
    { source: 'a'.repeat(DIFF_LIMIT + 1), start: 0, end: DIFF_LIMIT + 1, cursor: DIFF_LIMIT + 1, text: 'b'.repeat(DIFF_LIMIT + 1) },
    { source: 'a'.repeat(DIFF_LIMIT + 1), start: 0, end: DIFF_LIMIT + 1, cursor: DIFF_LIMIT + 1, text: 'a'.repeat(DIFF_LIMIT + 1) + 'b' },
].flatMap((sample) => MODES.map((mode) => ({ suffix: 0, ...sample, mode })))
const position = (source, offset) => {
    const lines = source.slice(0, offset).split(/\r\n|\n/)
    return new Position(lines.length, lines.at(-1).length + 1)
}
const offset = (lines, line, column) => lines.slice(0, line - 1).reduce((sum, value) => sum + value.length + 1, 0) + column - 1
const boundary = (source, at) =>
    !(
        at > 0 &&
        at < source.length &&
        source.charCodeAt(at - 1) >= 0xd800 &&
        source.charCodeAt(at - 1) <= 0xdbff &&
        source.charCodeAt(at) >= 0xdc00 &&
        source.charCodeAt(at) <= 0xdfff
    )
const hex = (value) => Buffer.from(value).toString('hex') || '_'
const rows = [...cases, ...special]
    .filter((sample) => [sample.start, sample.end, sample.cursor].every((at) => boundary(sample.source, at)))
    .map((sample) => {
        const lines = sample.source.split(/\r\n|\n/)
        const lf = lines.join('\n')
        const model = {
            getLineContent: (line) => lines[line - 1],
            getValueInRange: (range) =>
                lf.slice(offset(lines, range.startLineNumber, range.startColumn), offset(lines, range.endLineNumber, range.endColumn)),
        }
        const start = position(sample.source, sample.start)
        const end = position(sample.source, sample.end)
        const original = computeGhostText(
            new TextReplacement(Range.fromPositions(start, end), sample.text),
            model,
            sample.mode,
            position(sample.source, sample.cursor),
            sample.suffix,
        )
        const normalized = sample.text.replaceAll('\r\n', '\n')
        const originalRange = Range.fromPositions(start, end)
        const prefix = commonPrefixLength(model.getValueInRange(originalRange), normalized)
        const adjustedStart = position(lf, offset(lines, start.lineNumber, start.column) + prefix)
        const ghost = computeGhostText(
            new TextReplacement(Range.fromPositions(adjustedStart, end), normalized.slice(prefix)),
            model,
            sample.mode,
            position(sample.source, sample.cursor),
            sample.suffix,
        )
        const signature = (value) => (value ? [value.lineNumber, value.parts.map((part) => [part.column, part.text, part.preview])] : null)
        const corrected = JSON.stringify(signature(original)) !== JSON.stringify(signature(ghost))
        const lineStart = ghost
            ? sample.source
                  .split(/(?<=\n)/)
                  .slice(0, ghost.lineNumber - 1)
                  .reduce((sum, line) => sum + line.length, 0)
            : 0
        const unsafe = ghost?.parts.some((part) => !part.text.isWellFormed() || !boundary(sample.source, lineStart + part.column - 1)) ?? false
        const expected = ghost
            ? ghost.parts.map((part) => [lineStart + part.column - 1, hex(part.text), Number(part.preview)].join(':')).join(',') || '_'
            : '-'
        return [
            hex(sample.source),
            sample.start,
            sample.end,
            sample.cursor,
            hex(sample.text),
            sample.mode,
            sample.suffix,
            ghost?.lineNumber ?? 0,
            unsafe ? 'invalid-utf16' : expected,
            corrected ? 'utf16-prefix-correction' : 'original',
        ].join('\t')
    })
writeFileSync(resolve(ROOT, 'native/taide-native-editor/tests/fixtures/completion-preview-reference.tsv'), rows.join('\n') + '\n')
process.stdout.write(
    JSON.stringify({
        cases: rows.length,
        invalidUtf16: rows.filter((row) => row.includes('invalid-utf16')).length,
        corrected: rows.filter((row) => row.endsWith('utf16-prefix-correction')).length,
    }) + '\n',
)

const snippetBodies = [
    'func(${1:arg})$0',
    '${1:x}$1 ${2:later}',
    '$UNKNOWN ${KNOWN:default} ${EMPTY:}',
    '${1|red,green,blue|} $1',
    '${1:foo}${1/(.*)/${1:/upcase}/}',
    'a\n\tb',
    '  first\n\t  second\n${1:next}',
    'line\r\n\t\tend',
    '${VARIABLE/(.*)/${1:/upcase}/}',
    '${1:one${2:two}} $1',
    '\\$1 \\} \\backslashes',
    '$1 $1',
    'plain$1',
    '${1:a',
]
const snippetSources = ['', '  con', '\tcon', '    pre con', '한con', 'first\n\t\tcon']
const snippetCases = snippetSources.flatMap((source) =>
    snippetBodies.flatMap((body) =>
        [true, false].flatMap((isSnippet) =>
            [2, 4].flatMap((tabSize) =>
                [true, false].flatMap((insertSpaces) => ['\n', '\r\n'].map((eol) => ({ source, body, isSnippet, tabSize, insertSpaces, eol }))),
            ),
        ),
    ),
)
const snippetRows = snippetCases.map((sample) => {
    const lines = sample.source.split(/\r\n|\n/)
    const model = {
        getLineContent: (line) => lines[line - 1],
        getEOL: () => sample.eol,
        normalizeIndentation: (text) => normalizeIndentation(text, sample.tabSize, sample.insertSpaces),
    }
    const parsed = new SnippetParser().parse(sample.body)
    SnippetSession.adjustWhitespace(model, position(sample.source, sample.source.length), true, parsed)
    const expected = sample.isSnippet ? parsed.toString() : sample.body
    return [
        hex(sample.source),
        sample.source.length,
        hex(sample.body),
        Number(sample.isSnippet),
        sample.tabSize,
        Number(sample.insertSpaces),
        hex(sample.eol),
        hex(expected),
    ].join('\t')
})
writeFileSync(resolve(ROOT, 'native/taide-native-editor/tests/fixtures/completion-preview-snippets-reference.tsv'), snippetRows.join('\n') + '\n')
process.stdout.write(JSON.stringify({ snippetCases: snippetRows.length }) + '\n')

const viewSource = readFileSync(
    resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib/inlineCompletions/browser/view/ghostText/ghostTextView.js'),
    'utf8',
)
const viewStart = viewSource.indexOf('function computeGhostTextViewData(')
const viewEnd = viewSource.indexOf('\nclass AdditionalLinesData', viewStart)
if (viewStart < 0 || viewEnd < 0) throw new Error('Original ghost view function boundary changed')
const viewFactory = new Function(
    'LineDecoration',
    'ColumnRange',
    'RangeSingleLine',
    `${viewSource.slice(viewStart, viewEnd)}\nreturn computeGhostTextViewData`,
)
const computeView = viewFactory(LineDecoration, ColumnRange, RangeSingleLine)
const viewCases = [
    {
        source: 'abcdef',
        line: 1,
        parts: [
            [2, 'XX', false],
            [4, 'ZZ', false],
        ],
    },
    {
        source: 'abcdef',
        line: 1,
        parts: [
            [2, 'XX\nYY', false],
            [4, 'ZZ\nW', false],
        ],
    },
    {
        source: 'abc',
        line: 1,
        parts: [
            [3, 'x\ny', false],
            [3, 'z\nw', false],
            [3, '\n', false],
        ],
    },
    {
        source: '한\u{1f600}끝',
        line: 1,
        parts: [
            [1, 'α\n\t한', false],
            [3, ')\n다음', false],
        ],
    },
    {
        source: 'ab\r\ncdef',
        line: 2,
        parts: [
            [6, 'X\n\tY', false],
            [8, 'Z', false],
        ],
    },
    { source: '', line: 1, parts: [[0, '\nline\n', false]] },
    { source: 'abcdef', line: 1, parts: [[2, '', false]] },
    { source: 'abcdef', line: 1, parts: [] },
    {
        source: 'abcdef',
        line: 1,
        parts: [
            [2, 'x', false],
            [2, 'Y\nz', true],
        ],
    },
    {
        source: 'abcdef',
        line: 1,
        parts: [
            [0, 'x\r\ny\rz', false],
            [6, 'end', false],
        ],
    },
    { source: 'abcdef\nnext', line: 1, parts: [[6, ' tail\n  one\n\t\ttwo', false]] },
    {
        source: 'abcdef',
        line: 1,
        parts: [
            [0, '\n', false],
            [2, '\n', false],
            [4, '\n', false],
        ],
    },
]
const viewRows = viewCases.map((sample) => {
    const sourceLines = sample.source.split(/\r\n|\n/)
    const lineStart = sample.source
        .split(/(?<=\n)/)
        .slice(0, sample.line - 1)
        .reduce((sum, line) => sum + line.length, 0)
    const ghost = {
        lineNumber: sample.line,
        parts: sample.parts.map(([at, value, preview]) => ({
            column: at - lineStart + 1,
            lines: value.split(/\r\n|\r|\n/).map((line) => ({ line, lineDecorations: [] })),
            preview,
        })),
    }
    const view = computeView(ghost, { getLineContent: (line) => sourceLines[line - 1] }, 'ghost')
    const suffix = view.additionalLinesOriginalSuffix
    const additional =
        view.additionalLines
            .map((line, index) => {
                const originalSuffix =
                    suffix && index === view.additionalLines.length - 1
                        ? sourceLines[sample.line - 1].slice(suffix.columnRange.startColumn - 1, suffix.columnRange.endColumnExclusive - 1)
                        : ''
                return [
                    hex(line.content + originalSuffix),
                    line.decorations.map((decoration) => `${decoration.startColumn - 1}:${decoration.endColumn - 1}`).join(',') || '_',
                ].join(':')
            })
            .join(';') || '_'
    const parts = sample.parts.map(([at, value, preview]) => [at, hex(value), Number(preview)].join(':')).join(',') || '_'
    const inline = view.inlineTexts.map((part) => [lineStart + part.column - 1, hex(part.text), Number(part.preview)].join(':')).join(',') || '_'
    const hidden = view.hiddenRange ? `${lineStart + view.hiddenRange.startColumn - 1}:${lineStart + view.hiddenRange.endColumnExclusive - 1}` : '_'
    return [hex(sample.source), sample.line, parts, inline, hidden, additional].join('\t')
})
writeFileSync(resolve(ROOT, 'native/taide-native-editor/tests/fixtures/completion-preview-view-reference.tsv'), viewRows.join('\n') + '\n')
process.stdout.write(JSON.stringify({ viewCases: viewRows.length }) + '\n')
process.exit(0)
