import { resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

const config = resolveConfig({ type: 'markup', syntax: 'html' })
const literal = (value) => {
    if (value.includes('"###')) throw new Error('Rust raw string delimiter collision')
    return `r###"${value}"###`
}
const pairs = (name, values) =>
    `pub(crate) const ${name}: &[(&str, &str)] = &[\n${Object.entries(values)
        .map(([key, value]) => `    (${literal(key)}, ${literal(value)}),`)
        .join('\n')}\n];\n`
const array = (name, values) => `pub(crate) const ${name}: &[&str] = &[\n${values.map((value) => `    ${literal(value)},`).join('\n')}\n];\n`
const output = new URL('../../src/emmet-markup-defaults.rs', import.meta.url)
await Bun.write(
    output,
    pairs('SNIPPETS', config.snippets) + pairs('VARIABLES', config.variables) + array('INLINE_ELEMENTS', config.options.inlineElements),
)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet defaults formatting failed')
console.info(
    JSON.stringify({ engine: 'emmet@2.4.11', snippets: Object.keys(config.snippets).length, variables: Object.keys(config.variables).length }),
)
