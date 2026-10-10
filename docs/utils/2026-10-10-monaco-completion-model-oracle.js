import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { FuzzyScore } from '../../node_modules/monaco-editor/esm/vs/base/common/filters.js'
import { Position } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/position.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'
import { CompletionModel, LineContext } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/suggest/browser/completionModel.js'

const ROOT = resolve(import.meta.dir, '../..')
const suggestSource = readFileSync(resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib/suggest/browser/suggest.js'), 'utf8')
const itemStart = suggestSource.indexOf('class CompletionItem {')
const itemEnd = suggestSource.indexOf('class CompletionItemModel {')
const comparatorStart = suggestSource.indexOf('function defaultComparator(')
const comparatorEnd = suggestSource.indexOf('CommandsRegistry.registerCommand(', comparatorStart)
if (itemStart < 0 || itemEnd <= itemStart || comparatorStart < 0 || comparatorEnd <= comparatorStart) {
    throw new Error('Installed Monaco completion source boundaries changed')
}
const { CompletionItem, getSuggestionComparator } = new Function(
    'Position',
    'Range',
    'FuzzyScore',
    `${suggestSource.slice(itemStart, itemEnd)}\n${suggestSource.slice(comparatorStart, comparatorEnd)}\nreturn { CompletionItem, getSuggestionComparator }`,
)(Position, Range, FuzzyScore)
const TEXT_KIND = 18
const METHOD_KIND = 0
const FUNCTION_KIND = 1
const SNIPPET_KIND = 28
const LARGE_SOURCE_COUNT = 2010
const MATCHING_SOURCE_COUNT = 10
const SORT_INLINE = 1
const hex = (text) => Buffer.from(text).toString('hex')
const optional = (text) => (text === undefined ? '-' : hex(text))
const item = (label, overwrite, kind = TEXT_KIND, filterText, sortText) => ({ label, overwrite, kind, filterText, sortText })
const CASES = [
    {
        name: 'empty-raw-utf16-order',
        prefix: '',
        items: [item('a', 0), item('Z', 0), item('\ue000', 0), item('\u{1f600}', 0), item('a', 0, METHOD_KIND), item('a', 0, SNIPPET_KIND)],
        contexts: [
            ['', 0],
            ['a', 1],
            ['', 0],
        ],
    },
    {
        name: 'per-item-overwrite-filtertext',
        prefix: '    con',
        items: [
            item('Console', 3),
            item('console', 3),
            item('concat', 3),
            item('constant', 7),
            item('Other', 3, METHOD_KIND, 'console'),
            item('cOnSoLe', 3, SNIPPET_KIND, 'CONSOLE'),
            item('NoMatch', 3, TEXT_KIND, ''),
            item('fooBar', 0),
            item('Console', 3, FUNCTION_KIND),
            item('constructor', 3),
            item('continued', 3),
        ],
        contexts: [
            ['    con', 0],
            ['    cons', 1],
            ['    cnos', 1],
            ['    con', 0],
            ['    ', -3],
        ],
    },
    {
        name: 'sorttext-case-and-ties',
        prefix: '',
        items: [
            item('z', 0, TEXT_KIND, undefined, 'A'),
            item('b', 0, SNIPPET_KIND, undefined, 'a'),
            item('a', 0, METHOD_KIND, undefined, 'c'),
            item('a', 0, FUNCTION_KIND, undefined, 'C'),
        ],
        contexts: [
            ['', 0],
            ['a', 1],
            ['', 0],
        ],
    },
    {
        name: 'unicode-label-filter-highlights',
        prefix: '한\u{1f600}',
        items: [
            item('한\u{1f600}foo', 3),
            item('한\u{1f600}bar', 3, TEXT_KIND, '한\u{1f600}foo'),
            item('foo', 0),
            item('한글', 3),
            item('\u{1f600}word', 2),
        ],
        contexts: [
            ['한\u{1f600}', 0],
            ['한\u{1f600}f', 1],
            ['한\u{1f600}', 0],
        ],
    },
    {
        name: 'large-source-incremental-graceful-switch',
        prefix: 'c',
        items: Array.from({ length: LARGE_SOURCE_COUNT }, (_, index) => item(index < MATCHING_SOURCE_COUNT ? `console${index}` : `zoo${index}`, 1)),
        contexts: [
            ['c', 0],
            ['cnos', 3],
            ['c', 0],
        ],
    },
]
const rows = CASES.flatMap((testCase) => {
    const position = new Position(1, testCase.prefix.length + 1)
    const provider = {}
    const items = testCase.items
        .map(
            (value, index) =>
                new CompletionItem(
                    position,
                    {
                        label: value.label,
                        filterText: value.filterText,
                        sortText: value.sortText,
                        kind: value.kind,
                        insertText: value.label,
                        range: new Range(1, position.column - value.overwrite, 1, position.column),
                        data: String(index),
                    },
                    { incomplete: false },
                    provider,
                ),
        )
        .sort(getSuggestionComparator(SORT_INLINE))
    const model = new CompletionModel(
        items,
        position.column,
        new LineContext(testCase.prefix, 0),
        { distance: () => 0 },
        { filterGraceful: true },
        'inline',
    )
    const encodedItems = testCase.items
        .map((value) => [hex(value.label), optional(value.filterText), optional(value.sortText), value.kind, value.overwrite].join(','))
        .join(';')
    return testCase.contexts.map(([leading, delta]) => {
        model.lineContext = new LineContext(leading, delta)
        const expected = model.items
            .map((value) => [value.completion.data, value.score[0], value.score[1], value.score.slice(2).join('|')].join(','))
            .join(';')
        return [testCase.name, hex(testCase.prefix), encodedItems, hex(leading), delta, expected].join('\t')
    })
})
writeFileSync(resolve(ROOT, 'native/taide-native-editor/tests/fixtures/completion-model-reference.tsv'), `${rows.join('\n')}\n`)
process.stdout.write(`Monaco completion model: ${rows.length} context changes in ${CASES.length} scenarios\n`)
process.exit(0)
