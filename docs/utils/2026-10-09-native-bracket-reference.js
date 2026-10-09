import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { BracketPairsTree } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/bracketPairsTextModelPart/bracketPairsTree/bracketPairsTree.js'
import { LanguageBracketsConfiguration } from '../../node_modules/monaco-editor/esm/vs/editor/common/languages/supports/languageBracketsConfiguration.js'
import { GuidesTextModelPart } from '../../node_modules/monaco-editor/esm/vs/editor/common/model/guidesTextModelPart.js'
import { Range } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/range.js'

const TAB_SIZE = 4
const CORPUS = ['()[]{}', '{[\n}]\n)', '\n\t{\n  x\n\ty }\n', '<({})>', '<\n    { x }\n>', '漢字(\n\t值\n)😀', '\n\n x\n\n    y\n\n z\n\n']
const CONFIGURATION_PATH = new URL('../../native/taide-native-syntax/language-configurations/monaco.json', import.meta.url)
const OUTPUT_PATH = new URL('../../native/taide-native-syntax/tests/fixtures/bracket-display-reference.json', import.meta.url)
const configurations = JSON.parse(readFileSync(CONFIGURATION_PATH, 'utf8')).languages.filter((language) => language.configuration)
const byteRange = (lines, range) => {
    const byteAt = (line, column) =>
        lines.slice(0, line - 1).reduce((bytes, text) => bytes + Buffer.byteLength(text) + 1, 0) +
        Buffer.byteLength(lines[line - 1].slice(0, column - 1))
    return [byteAt(range.startLineNumber, range.startColumn), byteAt(range.endLineNumber, range.endColumn)]
}

const reference = (language, text) => {
    const configuration = language.configuration
    const lines = text.split('\n')
    const options = { tabSize: TAB_SIZE, indentSize: TAB_SIZE, bracketPairColorizationOptions: { independentColorPoolPerBracketType: false } }
    const bracketsNew = new LanguageBracketsConfiguration(language.languageId, configuration)
    const getLanguageConfiguration = () => ({ bracketsNew, foldingRules: configuration.folding })
    const model = {
        tokenization: { hasTokens: false },
        getValue: () => text,
        getLanguageId: () => language.languageId,
        getLineCount: () => lines.length,
        getLineLength: (line) => lines[line - 1].length,
        getLineContent: (line) => lines[line - 1],
        getLineMaxColumn: (line) => lines[line - 1].length + 1,
        getLineFirstNonWhitespaceColumn: (line) => {
            const first = lines[line - 1].search(/[^ \t]/)
            return first < 0 ? 0 : first + 1
        },
        getOptions: () => options,
    }
    const tree = new BracketPairsTree(model, getLanguageConfiguration)
    const fullRange = new Range(1, 1, lines.length, lines.at(-1).length + 1)
    model.bracketPairs = { getBracketPairsInRangeWithMinIndentation: (range) => tree.getBracketPairsInRange(range, true) }
    const guides = new GuidesTextModelPart(model, { getLanguageConfiguration })
    const brackets = tree
        .getBracketsInRange(fullRange, true)
        .toArray()
        .map((bracket) => [...byteRange(lines, bracket.range), bracket.nestingLevel, bracket.isInvalid])
    const pairs = tree
        .getBracketPairsInRange(fullRange, true)
        .toArray()
        .map((pair) => ({
            open: byteRange(lines, pair.openingBracketRange),
            close: pair.closingBracketRange ? byteRange(lines, pair.closingBracketRange) : null,
            level: pair.nestingLevel,
            minimumIndent: pair.minVisibleColumnIndentation,
        }))
    const indents = guides.getLinesIndentGuides(1, lines.length)
    const activeIndents = lines.map((_, index) => {
        const guide = guides.getActiveIndentGuide(index + 1, 1, lines.length)
        return [guide.startLineNumber - 1, guide.endLineNumber - 1, guide.indent]
    })
    tree.dispose()
    guides.dispose()
    return { languageId: language.languageId, text, brackets, pairs, indents, activeIndents }
}

const cases = configurations.flatMap((language) => {
    const pairs = language.configuration.brackets ?? []
    const nested = pairs.map(
        ([open, close], index) => `${open}\n\t${pairs.at(index - 1)?.[0] ?? ''} value ${pairs.at(index - 1)?.[1] ?? ''}\n${close}`,
    )
    return [...CORPUS, ...nested].map((text) => reference(language, text))
})
writeFileSync(OUTPUT_PATH, `${JSON.stringify({ version: '0.56.0', tabSize: TAB_SIZE, cases })}\n`)
console.info(JSON.stringify({ path: fileURLToPath(OUTPUT_PATH), languages: configurations.length, cases: cases.length }))
process.exit(0)
