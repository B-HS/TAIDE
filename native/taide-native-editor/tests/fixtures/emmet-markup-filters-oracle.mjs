import { doComplete } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'

import { rustLiteral } from './emmet-rust-literal.mjs'
import { fileURLToPath } from 'node:url'

const monaco = {
    Range: class {
        constructor(startLineNumber, startColumn, endLineNumber, endColumn) {
            Object.assign(this, { startLineNumber, startColumn, endLineNumber, endColumn })
        }
    },
    languages: { CompletionItemKind: { Property: 'property', Snippet: 'snippet' }, CompletionItemInsertTextRule: { InsertAsSnippet: 'snippet' } },
}
const stems = [
    'div.block',
    'div.block_mod',
    'div.block__element_mod',
    'div.b-block.other',
    'div.-element',
    'div._modifier',
    'div.block>.-element',
    'div.block>.-element_modifier',
    'div.block>.-element>._modifier',
    'div.block>.-element>.__modifier',
    'div.outer>div.inner>.--element_modifier',
    'div.outer>div.inner>.__modifier',
    'div.block.-element_modifier',
    'div.block.foo_bar',
    'div.block[class=other]',
    'div.block[class="foo bar"]',
    'div[class="${1:foo}"]',
    'div[class="${1}"]',
    'div.block[!class]',
    'div#main.foo',
    'div#main[ID=upper]',
    'div#main[ID]',
    'div.ID[class=""]',
    'div.[id=""]',
    'input#main.foo',
    'div.block>span.-element+span._modifier',
    'ul.block>li.-item*3',
    'div..block>.-element_modifier',
    'div[class="block\ufefffoo"]',
    'div[class="block\u0085foo"]',
    'div#${1}.block',
    'div#main>span.foo',
]
const suffixes = ['|bem', '|c', '|t', '|bem|c', '|c|bem', '|bem|t|c', '|c|c|c', '|bem|bem', '|c|t', '|t|c|bem']
const cases = ['html', 'jsx'].flatMap((syntax) =>
    stems.flatMap((stem) =>
        suffixes.map((suffix) => {
            const input = stem + suffix
            const model = { getLineContent: () => input }
            const result = doComplete(monaco, model, { lineNumber: 1, column: input.length + 1 }, syntax, {
                showExpandedAbbreviation: 'always',
                showAbbreviationSuggestions: true,
                showSuggestionsAsSnippets: false,
            })
            const primary = result?.suggestions.find((item) => item.detail === 'Emmet abbreviation')
            return `    (${rustLiteral(syntax)}, ${rustLiteral(input)}, ${primary ? `Some(${rustLiteral(primary.insertText)})` : 'None'}),`
        }),
    ),
)
const output = new URL('./emmet-markup-filters-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet filters oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet-monaco-es@5.7.0/emmet@2.4.11', stems: stems.length, suffixes: suffixes.length, cases: cases.length }))
