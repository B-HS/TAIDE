import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { Position } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/position.js'
import { BracketPairsTextModelPart } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/bracketPairsTextModelPart/bracketPairsImpl.js'
import { LanguageBracketsConfiguration } from '../../node_modules/monaco-editor/esm/vs/editor/common/languages/supports/languageBracketsConfiguration.js'

const TAB_SIZE = 4
const VERSION = '0.56.0'
const HEX_CODE_UNIT_WIDTH = 4
const CONFIGURATIONS = new URL('../../native/taide-native-syntax/language-configurations/monaco.json', import.meta.url)
const BASELINE = new URL('../../native/taide-native-syntax/tests/fixtures/bracket-display-reference.json', import.meta.url)
const OUTPUT = new URL('../../native/taide-native-syntax/tests/fixtures/bracket-matching-reference.json', import.meta.url)
const CORPUS = [
    '(())',
    '()[]',
    '[{}]',
    '({})',
    '{ [ } ]',
    '{ ( value }',
    '{ } ) (',
    'BEGIN\n END',
    '\\begin{value}\n\\end{value}',
    '\u{1f600}(()漢字)\u{1f600}',
    '(\n\n x\n)',
]
const configurations = JSON.parse(readFileSync(CONFIGURATIONS, 'utf8')).languages.filter((language) => language.configuration)
const baseline = JSON.parse(readFileSync(BASELINE, 'utf8'))
if (baseline.version !== VERSION) throw new Error('Bracket fixture version changed')
const configurationByLanguage = new Map(configurations.map((language) => [language.languageId, language.configuration]))

const reference = (languageId, text) => {
    const configuration = configurationByLanguage.get(languageId)
    if (!configuration) throw new Error(`Missing language configuration: ${languageId}`)
    const lines = text.split('\n')
    const bracketsNew = new LanguageBracketsConfiguration(languageId, configuration)
    const options = { tabSize: TAB_SIZE, indentSize: TAB_SIZE, bracketPairColorizationOptions: { independentColorPoolPerBracketType: false } }
    const model = {
        tokenization: { hasTokens: false },
        getValue: () => text,
        getValueLength: () => text.length,
        getLanguageId: () => languageId,
        getLineCount: () => lines.length,
        getLineLength: (line) => lines[line - 1].length,
        getLineContent: (line) => lines[line - 1],
        getLineMaxColumn: (line) => lines[line - 1].length + 1,
        getOptions: () => options,
        validatePosition: (position) => position,
    }
    const part = new BracketPairsTextModelPart(model, { getLanguageConfiguration: () => ({ bracketsNew }) })
    const byteAt = (line, column) =>
        lines.slice(0, line - 1).reduce((bytes, content) => bytes + Buffer.byteLength(content) + 1, 0) +
        Buffer.byteLength(lines[line - 1].slice(0, column - 1))
    const pair = (ranges) =>
        ranges?.map((range) => [byteAt(range.startLineNumber, range.startColumn), byteAt(range.endLineNumber, range.endColumn)]) ?? null
    const queries = []
    let byte = 0
    let line = 1
    let column = 1
    const query = () => {
        const position = new Position(line, column)
        const near = pair(part.matchBracket(position))
        const enclosing = pair(part.findEnclosingBrackets(position))
        queries.push({ byte, near, enclosing })
    }
    query()
    for (const character of text) {
        byte += Buffer.byteLength(character)
        if (character === '\n') {
            line += 1
            column = 1
        } else {
            column += character.length
        }
        query()
    }
    part.dispose()
    return { languageId, text, queries }
}

const inputs = [
    ...baseline.cases.map(({ languageId, text }) => ({ languageId, text })),
    ...configurations.flatMap(({ languageId }) => CORPUS.map((text) => ({ languageId, text }))),
]
const cases = inputs.map(({ languageId, text }) => reference(languageId, text))
const serialized = JSON.stringify({ version: VERSION, tabSize: TAB_SIZE, cases }).replace(/[\u{10000}-\u{10ffff}]/gu, (character) =>
    [0, 1].map((index) => `\\u${character.charCodeAt(index).toString(16).padStart(HEX_CODE_UNIT_WIDTH, '0')}`).join(''),
)
writeFileSync(OUTPUT, `${serialized}\n`)
console.info(
    JSON.stringify({
        path: fileURLToPath(OUTPUT),
        languages: configurations.length,
        cases: cases.length,
        queries: cases.reduce((count, item) => count + item.queries.length, 0),
    }),
)
process.exit(0)
