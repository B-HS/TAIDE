import parse from '@emmetio/css-abbreviation'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

import { canonicalStylesheet } from './emmet-stylesheet-canonical.mjs'

import { STYLESHEET_INPUTS, STYLESHEET_SNIPPET_COUNT } from './emmet-stylesheet-inputs.mjs'

const cases = [false, true].flatMap((isValue) =>
    STYLESHEET_INPUTS.map((input) => {
        let expected = 'None'
        try {
            expected = `Some(${rustLiteral(canonicalStylesheet(parse(input, { value: isValue })))})`
        } catch {}
        return `    (${isValue}, ${rustLiteral(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-stylesheet-parser-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(bool, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet stylesheet parser oracle formatting failed')
console.info(
    JSON.stringify({
        engine: '@emmetio/css-abbreviation@2.1.8',
        inputs: STYLESHEET_INPUTS.length,
        cases: cases.length,
        snippets: STYLESHEET_SNIPPET_COUNT,
    }),
)
