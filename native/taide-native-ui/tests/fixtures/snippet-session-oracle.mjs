import { IntervalNode, nodeAcceptEdit } from '../../../../node_modules/monaco-editor/esm/vs/editor/common/model/intervalTree.js'
import { Range } from '../../../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { normalizeIndentation } from '../../../../node_modules/monaco-editor/esm/vs/editor/common/core/misc/indentation.js'
import { SnippetParser, Transform } from '../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetParser.js'
import { OneSnippet, SnippetSession } from '../../../../node_modules/monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetSession.js'

const RANGE_CASES = [
    [0, 0],
    [0, 1],
    [0, 6],
    [1, 1],
    [1, 3],
    [1, 6],
    [3, 3],
    [3, 5],
    [3, 6],
    [5, 6],
    [6, 6],
    [0, 3],
]
const EDIT_CASES = [
    [0, 0],
    [0, 2],
    [1, 1],
    [1, 4],
    [3, 6],
    [6, 6],
    [0, 6],
]
const INSERTED_UNITS = [0, 1, 2, 4, 8]
const ALWAYS_GROWS = 0
const NEVER_GROWS = 1
const mapping = RANGE_CASES.flatMap(([start, end]) =>
    EDIT_CASES.flatMap(([editStart, editEnd]) =>
        INSERTED_UNITS.flatMap((inserted) =>
            [false, true].flatMap((grow) =>
                [false, true].map((force) => {
                    const node = new IntervalNode('synthetic', start, end)
                    node.setOptions({
                        className: null,
                        glyphMarginClassName: null,
                        stickiness: grow ? ALWAYS_GROWS : NEVER_GROWS,
                        collapseOnReplaceEdit: false,
                    })
                    nodeAcceptEdit(node, editStart, editEnd, inserted, force)
                    return { start, end, editStart, editEnd, inserted, grow, force, expected: [node.start, node.end] }
                }),
            ),
        ),
    ),
)

const configurations = [
    {
        template: '${1:foo} $1 ${2:bar}$0',
        actions: [{ type: '漢' }, { next: true }, { next: false }, { type: 'X' }, { next: true }, { type: 'Z' }, { next: true }],
    },
    { template: '${1:a${2:b}c} ${3:d}$0', actions: [{ type: 'Q' }, { next: true }, { type: 'R' }, { next: true }] },
    {
        template: '${1:pre ${2:x}}|$1|${3:end}$0',
        actions: [{ next: true }, { type: '𐐀' }, { next: false }, { next: true }, { next: true }, { next: true }],
    },
    { template: '${1|alpha,beta|} $1 ${2:end}$0', actions: [{ type: 'beta' }, { next: true }, { next: false }, { next: true }, { next: true }] },
    {
        template: '${1:foo} ${1/(.*)/${1:/upcase}/} ${2:end}$0',
        actions: [{ type: '\tfoo\n\tbar' }, { next: true }, { next: false }, { type: 'x' }, { next: true }, { next: true }],
    },
    { template: '${2:later}${1:𐐀} $1$0', actions: [{ type: '漢𐐀' }, { type: '字' }, { next: true }, { next: true }] },
    { template: '${1:}${2:next}$0', actions: [{ type: 'x' }, { next: true }, { type: '' }, { next: false }, { type: 'y' }, { next: true }] },
]
const contexts = [
    { leading: '  ', indentSize: 4, insertSpaces: true, eol: '\n' },
    { leading: '\t ', indentSize: 2, insertSpaces: false, eol: '\r\n' },
]

const originalResolve = Transform.prototype.resolve
let evaluations = []
class RecordingTransform extends Transform {
    resolve(value) {
        const result = originalResolve.call(this, value)
        evaluations = [...evaluations, { pattern: this.regexp.source, value, result }]
        return result
    }
}
Transform.prototype.resolve = RecordingTransform.prototype.resolve
let sessions
try {
    sessions = configurations.flatMap((configuration) =>
        contexts.map((context) => {
            evaluations = []
            const before = context.leading + 'TAIL' + context.eol
            let text = before
            let decorations = new Map()
            let sequence = 0
            let selections = []
            const positionAt = (offset) => {
                const lines = text.slice(0, offset).split(/\r\n|\r|\n/)
                return { lineNumber: lines.length, column: lines.at(-1).length + 1 }
            }
            const offsetAt = (position) => {
                let offset = 0
                const lines = text.split(/\r\n|\r|\n/)
                for (let line = 1; line < position.lineNumber; line++) offset += lines[line - 1].length + context.eol.length
                return offset + position.column - 1
            }
            const model = {
                getLineContent: (line) => text.split(/\r\n|\r|\n/)[line - 1],
                normalizeIndentation: (value) => normalizeIndentation(value, context.indentSize, context.insertSpaces),
                getEOL: () => context.eol,
                getPositionAt: positionAt,
                getValueInRange: (range) => text.slice(offsetAt(range.getStartPosition()), offsetAt(range.getEndPosition())),
                getDecorationRange: (id) => {
                    const node = decorations.get(id)
                    return node ? Range.fromPositions(positionAt(node.start), positionAt(node.end)) : null
                },
                changeDecorations: (callback) =>
                    callback({
                        addDecoration: (range, options) => {
                            sequence += 1
                            const id = String(sequence)
                            const node = new IntervalNode(id, offsetAt(range.getStartPosition()), offsetAt(range.getEndPosition()))
                            node.setOptions(options)
                            decorations.set(id, node)
                            return id
                        },
                        changeDecorationOptions: (id, options) => decorations.get(id).setOptions(options),
                    }),
            }
            const apply = (operations, shouldMoveSelections) => {
                const edits = operations
                    .map((operation) => ({
                        start: offsetAt(operation.range.getStartPosition()),
                        end: offsetAt(operation.range.getEndPosition()),
                        text: operation.text,
                        force: operation.forceMoveMarkers ?? false,
                    }))
                    .toSorted((left, right) => left.start - right.start || left.end - right.end)
                let removed = 0
                let inserted = 0
                const afterSelections = edits.map((edit) => {
                    const position = edit.start - removed + inserted + edit.text.length
                    removed += edit.end - edit.start
                    inserted += edit.text.length
                    return position
                })
                for (const edit of edits.toReversed()) {
                    for (const node of decorations.values()) nodeAcceptEdit(node, edit.start, edit.end, edit.text.length, edit.force)
                    text = text.slice(0, edit.start) + edit.text + text.slice(edit.end)
                }
                if (shouldMoveSelections) selections = afterSelections.map((offset) => Range.fromPositions(positionAt(offset)))
            }
            const editor = {
                hasModel: () => true,
                getModel: () => model,
                changeDecorations: model.changeDecorations,
                executeEdits: (_, operations) => apply(operations, false),
                removeDecorations: (ids) => {
                    for (const id of ids) decorations.delete(id)
                },
            }
            const snippet = new SnippetParser().parse(configuration.template, true, false)
            const leading = SnippetSession.adjustWhitespace(model, { lineNumber: 1, column: context.leading.length + 1 }, true, snippet)
            text = context.leading + snippet.toString() + 'TAIL' + context.eol
            const one = new OneSnippet(editor, snippet, leading)
            one.initialize({ newPosition: context.leading.length })
            selections = one.move(true)
            const snapshot = (action) => ({
                action,
                text,
                active: !one.isAtLastPlaceholder,
                selections: selections.map((selection) => [
                    Buffer.byteLength(text.slice(0, offsetAt(selection.getStartPosition()))),
                    Buffer.byteLength(text.slice(0, offsetAt(selection.getEndPosition()))),
                ]),
                choice: one.activeChoice?.choice.options.map((option) => option.value) ?? null,
                evaluations,
            })
            const initial = snapshot(null)
            const steps = configuration.actions.map((original) => {
                evaluations = []
                const action = typeof original.type === 'string' ? { type: original.type.replace(/\r\n|\r|\n/g, context.eol) } : original
                if (typeof action.type === 'string')
                    apply(
                        selections.map((range) => ({ range, text: action.type })),
                        true,
                    )
                else selections = one.move(action.next)
                return snapshot(action)
            })
            one.dispose()
            const retainedDecorations = decorations.size
            decorations = new Map()
            return { template: configuration.template, context, before, initial, steps, retainedDecorations }
        }),
    )
} finally {
    Transform.prototype.resolve = originalResolve
}
await new Promise((resolve, reject) => {
    process.stdout.write(JSON.stringify({ mapping, sessions }), (error) => {
        if (error) {
            reject(error)
            return
        }
        resolve()
    })
})
process.exit(0)
