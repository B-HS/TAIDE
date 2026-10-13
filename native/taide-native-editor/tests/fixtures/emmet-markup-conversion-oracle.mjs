import abbreviation from '@emmetio/abbreviation'
import { fileURLToPath } from 'node:url'

import { INPUTS } from './emmet-markup-inputs.mjs'

const MAX_REPEATS = 7
const VARIABLES = { lang: 'ko', charset: 'UTF-8', locale: 'ko-KR' }
const CASE_INPUTS = [
    ...INPUTS,
    '[]',
    '{text}[]>span',
    '{text}>span',
    '{${1:text}}>span',
    'li*0',
    'li*2000',
    'div*2>span.item$@^*3',
    '(a*2+b)*3',
    'div*+span*',
    'div*>{hello $#}',
    'div[foo="${lang}" title="${1:text}"]',
    'div{${charset} ${missing}}',
    'div..{styles.item}',
    'ul>li.item$$@-3*2',
    'p{\\${1}}',
]
const literal = (value) => {
    if (value.includes('"###')) throw new Error('Rust raw string delimiter collision')
    return `r###"${value}"###`
}
const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const values = (value) =>
    value == null
        ? '-'
        : `[${value.map((token) => (typeof token === 'string' ? `s${part(token)}` : `f${token.index}:${part(token.name)}`)).join('')}]`
const node = (value) => {
    const attributes =
        value.attributes == null
            ? '-'
            : `[${value.attributes.map((attribute) => `${part(attribute.name)}${values(attribute.value)}t${attribute.valueType}b${Number(attribute.boolean ?? false)}i${Number(attribute.implied ?? false)}m${Number(attribute.multiple ?? false)}`).join('')}]`
    const repeat = value.repeat ? `${value.repeat.count}:${value.repeat.value}:${Number(value.repeat.implicit)}` : '-'
    return `${part(value.name)}${values(value.value)}r${repeat}s${Number(value.selfClosing ?? false)}a${attributes}c[${value.children.map(node).join('')}]`
}
const cases = ['html', 'jsx'].flatMap((syntax) =>
    [false, true].flatMap((withVariables) =>
        CASE_INPUTS.map((input) => {
            let expected = 'None'
            try {
                const tree = abbreviation(input, { jsx: syntax === 'jsx', maxRepeat: MAX_REPEATS, variables: withVariables ? VARIABLES : {} })
                expected = `Some(${literal(tree.children.map(node).join(''))})`
            } catch {}
            return `    (${literal(syntax)}, ${withVariables}, ${literal(input)}, ${expected}),`
        }),
    ),
)
const output = new URL('./emmet-markup-conversion-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, bool, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet conversion oracle formatting failed')
console.info(JSON.stringify({ engine: '@emmetio/abbreviation@2.3.3', inputs: CASE_INPUTS.length, cases: cases.length, maxRepeat: MAX_REPEATS }))
