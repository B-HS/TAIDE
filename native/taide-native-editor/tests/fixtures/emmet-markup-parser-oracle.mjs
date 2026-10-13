import { rustLiteral as literal } from './emmet-rust-literal.mjs'
import { parse, tokenize } from '@emmetio/abbreviation'
import { fileURLToPath } from 'node:url'

import { INPUTS } from './emmet-markup-inputs.mjs'

const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const raw = (tokens, input) => tokens?.map((token) => (token.start == null ? token.value : input.slice(token.start, token.end))).join('')
const node = (value, input) => {
    const attributes =
        value.attributes
            ?.map(
                (attribute) =>
                    `${part(raw(attribute.name, input))}${part(raw(attribute.value, input))}x${Number(attribute.expression ?? false)}m${Number(attribute.multiple ?? false)}`,
            )
            .join('') ?? ''
    const repeat = value.repeat ? (value.repeat.implicit ? 'i' : String(value.repeat.count)) : '-'
    return `${value.type === 'TokenGroup' ? 'g' : 'e'}${part(raw(value.name, input))}${part(raw(value.value, input))}r${repeat}s${Number(value.selfClose ?? false)}a[${attributes}]c[${value.elements.map((child) => node(child, input)).join('')}]`
}
const cases = ['html', 'jsx'].flatMap((syntax) =>
    INPUTS.map((input) => {
        let expected = 'None'
        try {
            const tree = parse(tokenize(input), { jsx: syntax === 'jsx' })
            expected = `Some(${literal(tree.elements.map((value) => node(value, input)).join(''))})`
        } catch {}
        return `    (${literal(syntax)}, ${literal(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-markup-parser-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet parser oracle formatting failed')
console.info(JSON.stringify({ engine: '@emmetio/abbreviation@2.3.3', inputs: INPUTS.length, cases: cases.length }))
