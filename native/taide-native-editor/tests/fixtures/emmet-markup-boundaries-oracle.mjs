import { expandAbbreviation } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const MAX_REPEATS = 7
const inputs = [
    'p{\\\\}',
    'p{\\\\$cost}',
    'p{\\\\\\$cost}',
    'p{\\\\\\${1}}',
    'p{\\\\\\\\${1}}',
    'p{\\$cost}',
    'p{${1:cost$5}}',
    'p{${1:foo\\bar}}',
    'p{${1:a{b}c}}',
    'p{${1:${2:text}}}',
    'p{${1:foo\\}bar}}',
    'p{${1:foo\\\\bar}}',
    'p{${1:foo\\\\$cost}}',
    'p{${1:}}',
    'p{${0:first}${1:second}}',
    '${1:tag}>span',
    'div[${1:attr}=${2:value}]',
    'div[${0:attr}=${2:value}]',
    'div[foo="${1:${2:a{b}c}}"]',
    'div[foo="${1:a{b}c}"]',
    'div[foo="\\${1}"]',
    'div[foo="\\\\\\${1}"]',
    "div..foo'bar",
    "div..foo\\'bar",
    'div..foo\\\\bar',
    'div..foo-bar',
    'div..${1:foo}',
    'div..foo.${1:bar}',
    "div[title='a\"b']",
    'div[title="a\\"b"]',
    'p{a\r\nb\rc\nd}',
    'div.foo\u{1f642}',
]
const cases = ['html', 'jsx'].flatMap((syntax) =>
    inputs.map((input) => {
        let expected = 'None'
        try {
            const output = expandAbbreviation(input, {
                type: 'markup',
                syntax,
                maxRepeat: MAX_REPEATS,
                options: {
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
                },
            })
            expected = `Some(${rustLiteral(output)})`
        } catch {}
        return `    (${rustLiteral(syntax)}, ${rustLiteral(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-markup-boundaries-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet boundaries oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet@2.4.11', inputs: inputs.length, cases: cases.length }))
