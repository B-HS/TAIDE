import { readFileSync, writeFileSync } from 'node:fs'
import { ResolvedLanguageConfiguration } from '../../node_modules/monaco-editor/esm/vs/editor/common/languages/languageConfigurationRegistry.js'
import { LineTokens } from '../../node_modules/monaco-editor/esm/vs/editor/common/tokens/lineTokens.js'
import { getReindentEditOperations } from '../../node_modules/monaco-editor/esm/vs/editor/contrib/indentation/common/indentation.js'

const CONFIGURATION = 'native/taide-native-syntax/language-configurations/monaco.json'
const OUTPUT = 'native/taide-native-syntax/tests/fixtures/reindent-reference.json'
const TAB_SIZE = 4
const TOKEN_TYPE_OFFSET = 8
const STRING_TOKEN = 2
const COMMENT_TOKEN = 1
const TOKEN_LANGUAGE = 1
const languages = JSON.parse(readFileSync(CONFIGURATION, 'utf8'), (_, value) =>
    value?.source !== undefined && value?.flags !== undefined ? new RegExp(value.source, value.flags) : value,
).languages.filter((row) => row.configuration !== null)
const texts = [
    'if (x) {\nx();\nif (y) {\ny();\n}\n}',
    '  if (x) {\n\tx();\n       y();\n   }',
    'if value\nwork\nelse\nother\nend',
    'if value do\nwork\nend',
    'def work():\nx()\n\ny()',
    '# ignored\nif (x) {\n# ignored\nx();\n}',
    'if (x) {\n\n\nx();\n}',
    'if (x) {\r\n한글();\r\n\u{1f600}();\r\n}',
    'if (x) {\n    string {\nstring }\nx();\n}',
    'if (x) {\n// {\nx();\n}',
    'if (x)\nwork();\nrest();',
    '',
]
const cases = languages.flatMap(({ languageId, configuration }) => {
    const language = new ResolvedLanguageConfiguration(languageId, configuration)
    const service = { getLanguageConfiguration: () => language }
    const codec = { decodeLanguageId: () => languageId, encodeLanguageId: () => TOKEN_LANGUAGE }
    return texts.flatMap((text) => {
        const eol = text.includes('\r\n') ? '\r\n' : '\n'
        const lines = text.split(eol)
        const starts = lines.map((_, index) => lines.slice(0, index).reduce((offset, line) => offset + line.length + eol.length, 0))
        const tokenTypes = lines.map((line) => {
            if (line.includes('string ')) return STRING_TOKEN
            if (line.startsWith('//')) return COMMENT_TOKEN
            return 0
        })
        const selectionCases = [
            { selected: false, selections: [[text.length, text.length]] },
            { selected: true, selections: [[0, 0]] },
            { selected: true, selections: [[starts.at(-1), text.length]] },
            { selected: true, selections: [[0, starts[Math.min(2, starts.length - 1)]]] },
            {
                selected: true,
                selections: [
                    [text.length, 0],
                    [starts.at(-1), text.length],
                ],
            },
        ]
        return [true, false].flatMap((insertSpaces) =>
            selectionCases.map(({ selected, selections }) => {
                const model = {
                    getLineCount: () => lines.length,
                    getLineContent: (line) => lines[line - 1],
                    getLineMaxColumn: (line) => lines[line - 1].length + 1,
                    getLanguageId: () => languageId,
                    getOptions: () => ({ tabSize: TAB_SIZE, indentSize: TAB_SIZE, insertSpaces }),
                    tokenization: {
                        forceTokenization: () => {},
                        isCheapToTokenize: () => true,
                        getLineTokens: (line) =>
                            new LineTokens(
                                new Uint32Array([lines[line - 1].length, TOKEN_LANGUAGE | (tokenTypes[line - 1] << TOKEN_TYPE_OFFSET)]),
                                lines[line - 1],
                                codec,
                            ),
                    },
                }
                const ranges = selected
                    ? selections.flatMap(([anchor, head]) => {
                          const firstByte = Math.min(anchor, head)
                          const lastByte = Math.max(anchor, head)
                          const first = starts.findLastIndex((offset) => offset <= firstByte)
                          const last = starts.findLastIndex((offset) => offset <= lastByte)
                          const end = first < last && starts[last] === lastByte ? last : last + 1
                          return [[Math.max(0, first - 1) + 1, end]]
                      })
                    : [[1, lines.length]]
                const edits = ranges.flatMap(([first, last]) => getReindentEditOperations(model, service, first, last))
                const expected = lines
                    .map((line, index) => {
                        const edit = edits.find((edit) => edit.range.startLineNumber === index + 1)
                        return edit ? edit.text + line.slice(edit.range.endColumn - 1) : line
                    })
                    .join(eol)
                return {
                    language: languageId,
                    text,
                    token_types: tokenTypes,
                    selected,
                    selections: selections.map((range) => range.map((offset) => Buffer.byteLength(text.slice(0, offset)))),
                    tab_size: TAB_SIZE,
                    insert_spaces: insertSpaces,
                    expected,
                }
            }),
        )
    })
})
writeFileSync(OUTPUT, `${JSON.stringify(cases)}\n`)
console.info(JSON.stringify({ cases: cases.length, languages: languages.length, output: OUTPUT }))
process.exit(0)
