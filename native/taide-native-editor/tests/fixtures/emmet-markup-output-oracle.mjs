import { rustLiteral as literal } from './emmet-rust-literal.mjs'
import { resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'
import { expandAbbreviation } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'

import { INPUTS } from './emmet-markup-inputs.mjs'

const MAX_REPEATS = 7
const keys = Object.keys(resolveConfig({ type: 'markup', syntax: 'html' }).snippets)
const inputs = [
    ...new Set([
        ...INPUTS,
        ...keys,
        ...keys.map((name) => `${name}.class#id`),
        'ul>.a+.b',
        'ul>{${1}}>.a',
        'label>input',
        'label[for=foo]>input#bar',
        'div>span+strong',
        'div>span+section+span',
        'body',
        'html>head+body',
        'p{hello\nworld}',
        'div>{hello}+span',
        'div..foo',
        'div..for',
        'div..foo-bar',
        'p{\\${1}}',
        'p{\\$cost}',
        'p{${1:text}}>span',
        '{<section>text</section>}',
    ]),
]

const cases = ['html', 'jsx'].flatMap((syntax) =>
    inputs.map((input) => {
        let expected = 'None'
        try {
            const options = {
                'output.formatSkip': ['html'],
                'output.formatForce': ['body'],
                'output.field': (index, placeholder) => `\${${index}${placeholder ? ':' + placeholder : ''}}`,
                'output.inlineBreak': 0,
                'output.compactBoolean': false,
                'output.reverseAttributes': false,
                'markup.href': true,
                'jsx.enabled': syntax === 'jsx',
                'output.format': true,
                'output.selfClosingStyle': 'html',
            }
            expected = `Some(${literal(expandAbbreviation(input, { type: 'markup', syntax, maxRepeat: MAX_REPEATS, options }))})`
        } catch {}
        return `    (${literal(syntax)}, ${literal(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-markup-output-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet output oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet-monaco-es@5.7.0/emmet@2.4.11', inputs: inputs.length, cases: cases.length }))
