import { SnippetParser, Text, Variable } from '../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetParser.js'
import { Selection } from '../../../../node_modules/monaco-editor/esm/vs/editor/common/core/selection.js'
import {
    ClipboardBasedVariableResolver,
    SelectionBasedVariableResolver,
} from '../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetVariables.js'

const body = '  head𐐀\r\n    chosen\r\n  tail'
const start = Buffer.byteLength('  head𐐀\r\n  ')
const end = Buffer.byteLength('  head𐐀\r\n    chosen\r\n  ta')
const contexts = [
    { text: body, anchor: start, head: end, cursor: 0, count: 1, word: 'tail', overtyped: null },
    { text: body, anchor: end, head: start, cursor: 1, count: 2, word: 'chosen', overtyped: null },
    { text: '  owner\n', anchor: 1, head: 1, cursor: 0, count: 1, word: '', overtyped: { value: 'one\r\n\ttwo\r\n', multiline: true } },
    { text: '', anchor: 0, head: 0, cursor: 0, count: 1, word: null, overtyped: null },
    { text: 'last', anchor: 0, head: 0, cursor: 0, count: 1, word: 'last', overtyped: { value: '', multiline: false } },
    { text: body, anchor: start, head: start + Buffer.byteLength('  chosen'), cursor: 0, count: 1, word: 'chosen', overtyped: null },
]
const prefixes = ['', '\n\t\t', '${1|  choice,two|}', '${1: \t}\n    ']
const names = [
    'SELECTION',
    'TM_SELECTED_TEXT',
    'TM_CURRENT_LINE',
    'TM_CURRENT_WORD',
    'TM_LINE_INDEX',
    'TM_LINE_NUMBER',
    'CURSOR_INDEX',
    'CURSOR_NUMBER',
    'UNKNOWN',
]

const position = (text, byte) => {
    const before = Buffer.from(text).subarray(0, byte).toString()
    const lines = before.split(/\r\n|\r|\n/)
    return { line: lines.length, column: lines.at(-1).length + 1 }
}

const selectionCases = contexts.flatMap((context) =>
    prefixes.flatMap((prefix) =>
        names.map((name) => {
            const anchor = position(context.text, context.anchor)
            const head = position(context.text, context.head)
            const selection = new Selection(anchor.line, anchor.column, head.line, head.column)
            const lines = context.text.split(/\r\n|\r|\n/)
            const model = {
                getValueInRange: () =>
                    Buffer.from(context.text).subarray(Math.min(context.anchor, context.head), Math.max(context.anchor, context.head)).toString(),
                getLineContent: (line) => lines[line - 1],
                getWordAtPosition: () => (context.word === null ? null : { word: context.word }),
            }
            const resolver = new SelectionBasedVariableResolver(model, selection, context.cursor, {
                getLastOvertypedInfo: () => context.overtyped,
            })
            const snippet = new SnippetParser().parse(prefix + '$' + name)
            let variable = null
            let precedingTextLine = null
            snippet.walk((marker) => {
                if (marker instanceof Variable) {
                    variable = marker
                    return false
                }
                if (marker instanceof Text) precedingTextLine = marker.value.split(/\r\n|\r|\n/).at(-1)
                return true
            })
            if (!variable) throw new TypeError('Missing original variable')
            return { context, prefix, name, precedingTextLine, expected: resolver.resolve(variable) ?? null }
        }),
    ),
)

const clipboardContexts = [
    { text: null, count: 1, spread: true },
    { text: '', count: 1, spread: false },
    { text: 'one\r\n \n\t\r\n two \n', count: 2, spread: true },
    { text: 'one\r\n \n\t\r\n two \n', count: 2, spread: false },
    { text: 'one\ntwo', count: 3, spread: true },
    { text: '\u0085\n\u00a0\n\ufeff\n漢字', count: 2, spread: true },
    { text: '  ', count: 1, spread: true },
    { text: '\r\n\r\n', count: 2, spread: true },
]
const clipboardCases = clipboardContexts.flatMap((context) =>
    Array.from({ length: context.count }, (_, cursor) => ({
        ...context,
        cursor,
        expected:
            new ClipboardBasedVariableResolver(() => context.text, cursor, context.count, context.spread).resolve(new Variable('CLIPBOARD')) ?? null,
    })),
)

await new Promise((resolve, reject) => {
    process.stdout.write(JSON.stringify({ selectionCases, clipboardCases }), (error) => {
        if (error) {
            reject(error)
            return
        }
        resolve()
    })
})
process.exit(0)
