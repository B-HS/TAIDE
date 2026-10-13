import { doComplete } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'
import { cssData, htmlData } from '../../../../node_modules/emmet-monaco-es/src/data.ts'
import { resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const MULTIPLIER = 1664525
const INCREMENT = 1013904223
const RANDOM_SCALE = 2 ** 32
const SEED = 42
const monaco = {
    Range: class {
        constructor(startLineNumber, startColumn, endLineNumber, endColumn) {
            Object.assign(this, { startLineNumber, startColumn, endLineNumber, endColumn })
        }
    },
    languages: { CompletionItemKind: { Property: 'property', Snippet: 'snippet' }, CompletionItemInsertTextRule: { InsertAsSnippet: 'snippet' } },
}
const config = { showExpandedAbbreviation: 'always', showAbbreviationSuggestions: true, showSuggestionsAsSnippets: false }
const markupKeys = Object.keys(resolveConfig({ type: 'markup', syntax: 'html' }).snippets)
const cssKeys = Object.keys(resolveConfig({ type: 'stylesheet', syntax: 'css' }).snippets)
const inputs = [
    ...new Set([
        ...htmlData.tags,
        ...markupKeys,
        ...cssKeys,
        'a',
        'A',
        'ab',
        'abr',
        'd',
        'di',
        'div',
        'unknown',
        'Unknown',
        'MyComponent',
        '.',
        '#',
        '#id',
        '.class',
        'div.',
        'unknown.',
        'foo-bar',
        'foo:bar',
        'foo::bar',
        'foo--bar',
        'foo:',
        'foo,bar',
        '!',
        '!!',
        '!!!',
        '!foo',
        'ul>l',
        'ul>li',
        'div+i',
        'div+inp',
        'div.foo>in',
        'div.foo+in',
        'div.foo^in',
        'div.block>.-item|bem|c|t',
        '(word)',
        '(div)',
        '(div)+p',
        'div>(span)',
        'span[onclick="alert();"]',
        'p{(text)}',
        '(a+b)*2',
        '{hello}',
        '{hello (world)}',
        'div[foo=(bar)]',
        'm',
        'm10',
        'm10-20',
        'm:a',
        'd',
        'dib',
        'width',
        'wid',
        'font-family',
        'background',
        'backg',
        '!i',
        '!important',
        '!impo',
        '#f',
        '#ff',
        '#fff',
        '#ffffff',
        '#abcdefg',
        '#t',
        'div#id',
        'c#3',
        'lorem5',
        'p>lorem5*2',
        'lore',
        'text div',
        '<div',
        '<div.a',
        '<div>span',
        'prefix div|c',
        '\u{1f642} div',
        'div[title="x"]',
        'div[title="a\\"b"]',
        'm${variable}',
        'p{\\${1}}',
        'p{${1:cost$5}}',
        'ff',
        'f',
    ]),
]
const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const describe = (result) =>
    (result?.suggestions ?? [])
        .map((item) => {
            if (item.kind !== 'property' || item.insertTextRules !== 'snippet') throw new Error('Unexpected source Emmet item contract')
            return `${part(item.label)}${part(item.detail)}${part(item.insertText)}${part(item.documentation)}${part(item.filterText)}${part(item.sortText)}r${item.range.startColumn - 1}:${item.range.endColumn - 1}`
        })
        .join('')
const cases = []
const original = Math.random
try {
    for (const syntax of ['html', 'jsx', 'css']) {
        for (const line of inputs) {
            const boundaries = [...new Set([line.length, Math.max(0, line.length - 1), 0])].filter(
                (cursor) => cursor === line.length || !/[\uDC00-\uDFFF]/.test(line[cursor]),
            )
            for (const cursor of boundaries) {
                let state = SEED
                Math.random = () => {
                    state = (Math.imul(state, MULTIPLIER) + INCREMENT) >>> 0
                    return state / RANDOM_SCALE
                }
                const result = doComplete(monaco, { getLineContent: () => line }, { lineNumber: 1, column: cursor + 1 }, syntax, config)
                const bytes = new TextEncoder().encode(line.slice(0, cursor)).length
                cases.push(`    (${rustLiteral(syntax)}, ${rustLiteral(line)}, ${bytes}, ${rustLiteral(describe(result))}),`)
            }
        }
    }
} finally {
    Math.random = original
}
const output = new URL('./emmet-completion-oracle.rs', import.meta.url)
const defaultsPath = new URL('../../src/emmet-completion-defaults.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, usize, &str)] = &[\n${cases.join('\n')}\n];\n`)
const arrays = [
    ['HTML_TAGS', htmlData.tags],
    ['COMMON_TAGS', [...htmlData.tags, 'lorem']],
    ['MARKUP_KEYS', markupKeys],
    ['CSS_PROPERTIES', cssData.properties],
]
    .map(([name, values]) => `pub const ${name}: &[&str] = &[${values.map(rustLiteral).join(', ')}];`)
    .join('\n')
await Bun.write(defaultsPath, arrays + '\n')
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output), fileURLToPath(defaultsPath)])
if (formatted.exitCode !== 0) throw new Error('Emmet completion oracle formatting failed')
console.info(
    JSON.stringify({
        engine: 'emmet-monaco-es@5.7.0',
        inputs: inputs.length,
        cases: cases.length,
        htmlTags: htmlData.tags.length,
        cssProperties: cssData.properties.length,
    }),
)
