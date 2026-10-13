import { rustLiteral as literal } from './emmet-rust-literal.mjs'
import { parseMarkup, resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

import { INPUTS } from './emmet-markup-inputs.mjs'
import { canonicalNodes } from './emmet-markup-canonical.mjs'

const MAX_REPEATS = 7
const snippets = Object.keys(resolveConfig({ type: 'markup', syntax: 'html' }).snippets)
const inputs = [
    ...new Set([
        ...INPUTS,
        ...snippets,
        ...snippets.map((name) => `${name}.class#id`),
        'ul>.a+.b',
        'ul>{${1}}>.a',
        'P>.a',
        'a>.a',
        'label>input',
        'label[for=foo]>input#bar',
        'table>.row>.cell',
        'div.class[class=other]',
        'div[foo=first][foo=last]',
        'div[!data-x][data-x=value]',
        'div[foo.][foo=x]',
        'div..foo.class',
        '[]',
        '(input:checkbox+img)*2',
    ]),
]

const cases = ['html', 'jsx'].flatMap((syntax) =>
    inputs.map((input) => {
        let expected = 'None'
        try {
            const config = resolveConfig({ type: 'markup', syntax, maxRepeat: MAX_REPEATS })
            const tree = parseMarkup(input, config)
            expected = `Some(${literal(canonicalNodes(tree.children))})`
        } catch {}
        return `    (${literal(syntax)}, ${literal(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-markup-preparation-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet preparation oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet@2.4.11', inputs: inputs.length, cases: cases.length, snippets: snippets.length }))
