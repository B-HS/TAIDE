import { parseStylesheet, resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

import { canonicalStylesheet } from './emmet-stylesheet-canonical.mjs'
import { PREPARATION_INPUTS } from './emmet-stylesheet-preparation-inputs.mjs'
import { rustLiteral } from './emmet-rust-literal.mjs'

import { FUZZY_MIN_SCORE, FUZZY_SCORE_TEXT, createStylesheetOptions } from './emmet-stylesheet-config.mjs'

const config = resolveConfig({ type: 'stylesheet', syntax: 'css', options: createStylesheetOptions() })
const entries = Object.entries(config.snippets).toSorted(([left], [right]) => (left < right ? -1 : Number(left > right)))
const table = `pub const SNIPPETS: &[(&str, &str)] = &[\n${entries.map(([key, value]) => `    (${rustLiteral(key)}, ${rustLiteral(value)}),`).join('\n')}\n];\n`
const arrays = ['unitless', 'keywords']
    .map((key) => `pub const ${key.toUpperCase()}: &[&str] = &[${config.options['stylesheet.' + key].map(rustLiteral).join(', ')}];`)
    .join('\n')
const aliases = `pub const UNIT_ALIASES: &[(&str, &str)] = &[${Object.entries(config.options['stylesheet.unitAliases'])
    .map(([key, value]) => `(${rustLiteral(key)}, ${rustLiteral(value)})`)
    .join(', ')}];`
const units = ['intUnit', 'floatUnit']
    .map((key) => `pub const ${key === 'intUnit' ? 'INTEGER_UNIT' : 'FLOAT_UNIT'}: &str = ${rustLiteral(config.options['stylesheet.' + key])};`)
    .join('\n')
const defaultsPath = new URL('../../src/emmet-stylesheet-defaults.rs', import.meta.url)
await Bun.write(defaultsPath, [table, arrays, aliases, units, `pub const FUZZY_MIN_SCORE: f64 = ${FUZZY_SCORE_TEXT};`, ''].join('\n'))
const cases = PREPARATION_INPUTS.map((input) => {
    let expected = 'None'
    try {
        expected = `Some(${rustLiteral(canonicalStylesheet(parseStylesheet(input, config)))})`
    } catch {}
    return `    (${rustLiteral(input)}, ${expected}),`
})
const output = new URL('./emmet-stylesheet-preparation-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(defaultsPath), fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet stylesheet preparation oracle formatting failed')
console.info(
    JSON.stringify({
        engine: 'emmet@2.4.11/emmet-monaco-es@5.7.0',
        fuzzyMinScore: FUZZY_MIN_SCORE,
        inputs: PREPARATION_INPUTS.length,
        cases: cases.length,
        snippets: entries.length,
    }),
)
