import { writeFileSync } from 'node:fs'
import { FormattingEdit } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/format/browser/formattingEdit.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { IntervalNode, nodeAcceptEdit } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/intervalTree.js'

const OUTPUT = 'native/taide-native-editor/tests/fixtures/formatting-markers-reference.txt'
const REPLACEMENT_WIDTH = 1000
const SELECTION_START = 77
const SELECTION_END = 113
const CASES = [
    ['a'.repeat(REPLACEMENT_WIDTH), 'b'.repeat(REPLACEMENT_WIDTH)],
    ['a'.repeat(REPLACEMENT_WIDTH), 'b'.repeat(REPLACEMENT_WIDTH / 2)],
    ['a'.repeat(REPLACEMENT_WIDTH), 'b'.repeat(REPLACEMENT_WIDTH + 1)],
    [`한\u{1f600}${'a'.repeat(REPLACEMENT_WIDTH)}`, `한\u{1f600}${'b'.repeat(REPLACEMENT_WIDTH)}`],
]
const rows = CASES.map(([text, replacement]) => {
    const selections = [
        [0, 0],
        [SELECTION_START, SELECTION_START],
        [SELECTION_END, SELECTION_START],
        [text.length, text.length],
    ]
    const nodes = selections.map(([anchor, head], index) => {
        const node = new IntervalNode(String(index), Math.min(anchor, head), Math.max(anchor, head))
        node.setOptions({ stickiness: 0, collapseOnReplaceEdit: false, glyphMarginClassName: null })
        return node
    })
    const range = new Range(1, 1, 1, text.length + 1)
    const editor = {
        hasModel: () => true,
        getModel: () => ({ validateRange: (value) => value, getFullModelRange: () => range }),
        pushUndoStop: () => {},
        getScrollTop: () => 0,
        getContentHeight: () => 0,
        getPosition: () => null,
        executeEdits: (_, edits) => {
            for (const edit of edits) {
                for (const node of nodes) nodeAcceptEdit(node, 0, text.length, edit.text.length, edit.forceMoveMarkers ?? false)
            }
        },
    }
    FormattingEdit.execute(editor, [{ range, text: replacement }], true)
    const bytes = (value, offset) => Buffer.byteLength(value.slice(0, offset))
    return {
        text,
        replacement,
        selections: selections.map(([anchor, head]) => [bytes(text, anchor), bytes(text, head)]),
        expected: nodes.map((node, index) => {
            const [anchor, head] = selections[index]
            if (anchor === head) return [bytes(replacement, node.end), bytes(replacement, node.end)]
            const offsets = anchor < head ? [node.start, node.end] : [node.end, node.start]
            return offsets.map((offset) => bytes(replacement, offset))
        }),
    }
})
const offsets = (selections) => selections.map((selection) => selection.join(',')).join(';')
writeFileSync(OUTPUT, `${rows.map((row) => [row.text, row.replacement, offsets(row.selections), offsets(row.expected)].join('\t')).join('\n')}\n`)
console.info(JSON.stringify({ cases: rows.length, output: OUTPUT }))
