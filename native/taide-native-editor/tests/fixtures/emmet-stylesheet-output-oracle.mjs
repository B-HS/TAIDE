import { expandAbbreviation } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'
import { fileURLToPath } from 'node:url'

import { PREPARATION_INPUTS } from './emmet-stylesheet-preparation-inputs.mjs'
import { rustLiteral } from './emmet-rust-literal.mjs'

import { createStylesheetOptions } from './emmet-stylesheet-config.mjs'

const [minimumSignificand, minimumExponent] = Number.MIN_VALUE.toExponential().split('e-')
const minimumDecimal = '0.' + '0'.repeat(Number(minimumExponent) - 1) + minimumSignificand
const inputs = [
    ...new Set([
        ...PREPARATION_INPUTS,
        'm' + minimumDecimal,
        'm-' + minimumDecimal,
        'm0.03125',
        'm-0.03125',
        'm0.00005',
        'm-0.00005',
        'm0.00015',
        'm0.00001',
        'm-0.00001',
        'm1000000000000000128',
        'm1000000000000000000000',
        'm1000000000000000000000000000000',
        'm1.00005',
        'm1.005',
        'm0.000000000000000000000000000000000000001',
        'c#abcdef',
        'c#ababab',
        'c#1234',
        'c#12345678',
        'c#t',
        'c#t.5',
        'c#000.0',
        'c#fff.5',
        'c#fff.123456785',
        'c#fff.123456795',
        'c#fff.000000005',
        'c#fff.001953125',
        'm${1:cost$5}',
        'm${1:a{b}c}',
        'm${1:foo\\}bar}',
        'm${foo:bar}',
        'm${foo}+p${bar}',
        "f:'${1:FontName}'",
        "f:'${1}'",
        "f:'\\\\${1}'",
        "f:'\\$cost'",
        "f:'\\${1}'",
        "f:'a\r\nb'",
        'm${1:foo\\\\bar}',
    ]),
]
const cases = ['css', 'scss'].flatMap((syntax) =>
    inputs.map((input) => {
        let expected = 'None'
        try {
            const output = expandAbbreviation(input, {
                type: 'stylesheet',
                syntax,
                options: createStylesheetOptions(),
            })
            expected = `Some(${rustLiteral(output)})`
        } catch {}
        return `    (${rustLiteral(syntax)}, ${rustLiteral(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-stylesheet-output-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet stylesheet output oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet-monaco-es@5.7.0/emmet@2.4.11', inputs: inputs.length, cases: cases.length }))
