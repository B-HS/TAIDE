import { parseStylesheet, resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

import { canonicalStylesheet } from './emmet-stylesheet-canonical.mjs'
import { STYLESHEET_INPUTS } from './emmet-stylesheet-inputs.mjs'
import { rustLiteral } from './emmet-rust-literal.mjs'

const helper = await Bun.file(new URL('../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts', import.meta.url)).text()
const fuzzyText = helper.match(/'stylesheet\.fuzzySearchMinScore':\s*(0(?:\.\d+)?)/)?.[1]
if (!fuzzyText) throw new Error('Missing project Emmet fuzzy threshold')
const fuzzyMinScore = Number(fuzzyText)
const config = resolveConfig({ type: 'stylesheet', syntax: 'css', options: { 'stylesheet.fuzzySearchMinScore': fuzzyMinScore } })
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
await Bun.write(defaultsPath, [table, arrays, aliases, units, `pub const FUZZY_MIN_SCORE: f64 = ${fuzzyText};`, ''].join('\n'))
const inputs = [
    ...new Set([
        ...STYLESHEET_INPUTS,
        ...entries.flatMap(([key]) => [key + ':a', key + ':n', key + ':s', key + ':i', key + ':u']),
        'dib',
        'd-ib',
        'poa',
        'posr',
        'pos:a',
        'bds',
        'bd:so',
        'bg:red',
        'bgn',
        'margin',
        'width',
        'font-family',
        'unknown12',
        'w10',
        'm1.5',
        'm0',
        'lh2',
        'fz1.',
        'z10',
        'flex2',
        'zoom3',
        'm10r',
        'm10foo',
        'lg',
        'lg(red,blue)',
        'lg(45deg,red,blue)',
        'bg:lg(red,blue)',
        'lg+bg#fff',
        '@m10',
        '@i"x"',
        'd10${1}',
        'd-b${1}',
        'd10-${1}',
        'bg-n${1}',
        'bg-none${1}',
        'f:foo(1)${1}',
    ]),
]
const cases = inputs.map((input) => {
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
        fuzzyMinScore,
        inputs: inputs.length,
        cases: cases.length,
        snippets: entries.length,
    }),
)
