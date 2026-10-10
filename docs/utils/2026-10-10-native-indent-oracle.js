import { guessIndentation } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/indentationGuesser.js'

const FIXTURE_PATH = 'native/taide-native-editor/tests/fixtures/indentation-guesses.tsv'
const DEFAULTS = [
    { tabSize: 2, insertSpaces: true },
    { tabSize: 4, insertSpaces: true },
    { tabSize: 8, insertSpaces: true },
    { tabSize: 4, insertSpaces: false },
    { tabSize: 8, insertSpaces: false },
]
const CASES = [
    ['empty', ''],
    ['flat', 'root\nchild\nend'],
    ['whitespace', ' \n\t\n    \n\r\n'],
    ['one-space', 'a\n b\n c'],
    ['two-space', 'a\n  b\n    c\n  d\ne'],
    ['four-space', 'a\n    b\n        c\n    d\ne'],
    ['three-space', 'a\n   b\n      c\n   d\ne'],
    ['six-space', 'a\n      b\n            c\ne'],
    ['eight-space', 'a\n        b\n                c\ne'],
    ['tabs', 'a\n\tb\n\t\tc\n\td\ne'],
    ['tied-style', 'a\n\tb\n    c\nd'],
    ['mixed-prefix', 'a\n \tb\n  \t c\n    d\ne'],
    ['mixed-diff', '\ta\n    b\n\t\tc\n        d'],
    ['alignment', 'const a = b + c,\n      d = b - c;\nroot\n    child'],
    ['list-alignment', '- a,\n  - b,\n    - c'],
    ['unicode', '\u{1f600}a\n  한b\n    \u{1f600}c\n  d\ne'],
    ['crlf', 'a\r\n  b\r\n    c\r\n  d\r\ne\r\n'],
    ['cr', 'a\r  b\r    c\r  d\re\r'],
    ['blank-between', 'a\n  b\n        \n\t\n    c\n  d\ne'],
    ['two-over-four', 'a\n    b\nc\n    d\ne\n    f\ng\n  h\ni\n  j'],
]

const rows = CASES.flatMap(([name, content]) => {
    const lines = content.split(/\r\n|\r|\n/)
    const source = {
        getLineCount: () => lines.length,
        getLineLength: (line) => lines[line - 1].length,
        getLineContent: (line) => lines[line - 1],
        getLineCharCode: (line, column) => lines[line - 1].charCodeAt(column),
    }
    return DEFAULTS.map((defaults) => {
        const result = guessIndentation(source, defaults.tabSize, defaults.insertSpaces)
        return [name, Buffer.from(content).toString('hex'), defaults.tabSize, defaults.insertSpaces, result.tabSize, result.insertSpaces].join('\t')
    })
})

await Bun.write(FIXTURE_PATH, `${rows.join('\n')}\n`)
process.stdout.write(`${rows.length} Monaco indentation fixtures written\n`)
