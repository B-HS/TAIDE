import parse from '@emmetio/css-abbreviation'
import { resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const part = (value) => (value == null ? '-' : `${new TextEncoder().encode(value).length}:${value}`)
const valueOf = (value) => {
    const kinds = {
        Literal: () => `l${part(value.value)}`,
        CustomProperty: () => `c${part(value.value)}`,
        NumberValue: () => `n${value.value}:${part(value.rawValue)}${part(value.unit)}`,
        ColorValue: () => `r${value.r}:${value.g}:${value.b}:${value.a}:${part(value.raw)}`,
        StringValue: () => `s${value.quote}:${part(value.value)}`,
        Field: () => `f${value.index == null ? '-' : value.index}:${part(value.name)}`,
        FunctionCall: () => `x${part(value.name)}[${fragmentsOf(value.arguments)}]`,
    }
    const render = kinds[value.type]
    if (!render) throw new Error(`Unknown CSS value ${value.type}`)
    return render()
}
const fragmentsOf = (fragments) => fragments.map((fragment) => `[${fragment.value.map(valueOf).join('')}]`).join('')
const canonical = (properties) =>
    properties.map((property) => `${part(property.name)}i${Number(property.important)}[${fragmentsOf(property.value)}]`).join('')
const snippets = resolveConfig({ type: 'stylesheet', syntax: 'css' }).snippets
const inputs = [
    ...new Set([
        ...Object.keys(snippets),
        ...Object.keys(snippets).map((key) => key + '10'),
        ...Object.values(snippets),
        '',
        'p10+poa',
        'm10-20',
        'm-10--20',
        'm10ex-5',
        'm1.5e20p',
        'm-.5',
        'm1.',
        'm.',
        'm0',
        '#',
        '#x',
        '#t',
        '#t.5',
        '#.5',
        '#f.',
        '#1234',
        '#abcdef00',
        'c#3',
        'bd5#0s',
        'rgb(1,2,3)',
        'rgba(1 2 3,.5)',
        'transform:scale3d(1,2,3)',
        'f:calc(100%-2px)',
        'f:var(--foo,red)',
        'f:url("a/b.png")',
        'f:foo(bar(1,2),3)',
        'foo(',
        'foo)',
        '()',
        '+',
        '++m10+',
        '!',
        'm10!+p!5',
        'm 10',
        ' 10 20 ',
        'm\u00a010',
        'm\u008510',
        'm\ufeff10',
        'm中文',
        '--foo',
        '--foo-bar10',
        '$var10',
        'm$var10',
        '@foo10',
        'm${1:20px}',
        'm${variable}',
        'm${1:a{b}c}',
        'm${1}',
        'm${}',
        'm${1:}',
        'm${1',
        "f:'open",
        "f:'a\\'b'",
        '.foo',
        '/foo',
        ':',
        '-',
        'foo(1!2)',
    ]),
]
const cases = [false, true].flatMap((isValue) =>
    inputs.map((input) => {
        let expected = 'None'
        try {
            expected = `Some(${rustLiteral(canonical(parse(input, { value: isValue })))})`
        } catch {}
        return `    (${isValue}, ${rustLiteral(input)}, ${expected}),`
    }),
)
const output = new URL('./emmet-stylesheet-parser-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(bool, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet stylesheet parser oracle formatting failed')
console.info(
    JSON.stringify({ engine: '@emmetio/css-abbreviation@2.1.8', inputs: inputs.length, cases: cases.length, snippets: Object.keys(snippets).length }),
)
