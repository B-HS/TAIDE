import { parse, tokenize } from '@emmetio/abbreviation'
import { resolveConfig } from 'emmet'
import { fileURLToPath } from 'node:url'

const ASCII_PRINTABLE_START = 32
const ASCII_PRINTABLE_COUNT = 95

const BASE_INPUTS = [
    '',
    'div',
    'custom-element',
    '.foo',
    '#main',
    'div.foo.bar#main',
    'ul>li.item$*3',
    'div>span+strong',
    'div>p^aside',
    'div>p^^aside',
    '(header+main)>p',
    'div>(header+main)>p',
    '(header+main)*2',
    'div>(ul>li*2)+footer',
    '((a+b)>c)*3',
    'div>span{hello}+em{world}',
    'span{한글 😀}',
    'span{a{b}c}',
    '{text}>span',
    'div[hidden]',
    'div[foo=bar bar="hello world"]',
    "a[href='https://example.test']",
    'input:checkbox',
    'a[href=/docs]{Link}',
    '[disabled]',
    'div[data-x={value}]',
    'div[foo=(bar)]',
    'div[foo=[bar]]',
    'div["default value"]',
    'Foo.Bar.Baz',
    'Foo.Bar.baz',
    'Component.{styles.item}',
    'div..foo',
    'div##main',
    'div[title="a\\"b"]',
    'div{a\\{b}',
    'li.item$$@3*2',
    'li.item$@-3*2',
    'div/${1}',
    'br/',
    'div*',
    'div*0',
    'div*2.foo',
    'div{hello}*2',
    '(a+b)>c^d',
    'div>^p',
    'div+',
    'div>',
    'div[foo=]',
    'div[',
    'div{',
    '(div',
    'div)',
    'div[title="',
    'div ${1}',
    'div${1}',
    'h$*3',
    'div[!data-x]',
    'div[hidden.]',
    'ul>li[title=$]{item $}*2',
    'p{${1:text}}',
    'div[a="${1:hello}"]',
    'a+b+c+d',
    'div>(a+b)^c',
    'div[foo]#id[bar=x].class',
]
const INPUTS = [
    ...new Set([
        ...BASE_INPUTS,
        ...Object.values(resolveConfig({ type: 'markup', syntax: 'html' }).snippets),
        'div[foo=a+b*c#d.e/f]',
        'div[a="brace } and quote"]',
        'div.{styles.item}#main',
        'Foo.Bar.Baz.class',
        'div[foo=""]',
        'div[foo=hello\\ world]',
        'div[foo={a: "}"}]',
        '(a+b)/(c+d)',
        'div.w-1/2.md:w-1/3',
        'div[style="width:100%"]',
        'li*2/',
        ...Array.from({ length: ASCII_PRINTABLE_COUNT }, (_, index) => String.fromCharCode(index + ASCII_PRINTABLE_START)).flatMap((character) => [
            `div${character}span`,
            `.a${character}b`,
            `div[attr=a${character}b]`,
        ]),
    ]),
]
const literal = (value) => {
    if (value.includes('"###')) throw new Error('Rust raw string delimiter collision')
    return `r###"${value}"###`
}
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
