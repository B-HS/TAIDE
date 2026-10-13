import { rustLiteral as literal } from './emmet-rust-literal.mjs'
import { extract } from 'emmet'
import { fileURLToPath } from 'node:url'

const INPUTS = [
    '',
    'div',
    'div.foo#main',
    'ul>li.item$*3',
    'div>(header>nav)+main+footer',
    'div>span+strong',
    'div>p^aside',
    'span{한글 \u{1f600} (text)}',
    'div{a{b}c}',
    'span[title="hello world"]',
    "a[href='https://example.test/a?q=1']",
    'input[disabled]',
    'span[data-x={value}]',
    'div[title="a\\"b"]',
    'div[title="x"]>span{hello}',
    '(ul>li)*2',
    'span(hello)',
    '>>>div',
    '***',
    '<div>span',
    '<div class="a b">span',
    '<input disabled/>.thing',
    '<div data-value={x}>.child',
    '<div data-x=foo(bar)>.child',
    '<div data-x=foo[bar]>.child',
    '<div data-x=foo{bar}>.child',
    '<div data-x=foo(bar) other="hello">span',
    '<ns:component/>div',
    '</div>span',
    '<>span',
    '<invalid title=>span',
    '<span>.foo[title=bar]</span>',
    '<a href=""><span>ul>li</span></a>',
    '한글 div',
    '\u{1f600} div>span',
    'x = div',
    'hello\tul>li',
    'div[title="한글 \u{1f600}"]',
    'div{你好(\u{1f600})}',
    'div[',
    'div]',
    '(div',
    'div)',
    'div{',
    'div}',
    'div[title="',
    'div[title=""]',
    'div[a=(foo)]',
    'm10',
    'p10-20',
    'fz1.5',
    'c#fff',
    '#abc',
    '-webkit-foo',
    '.test { m10 }',
    'display: flex;',
    'a:hover',
    'background:url(test.png)',
    'transform: translate(10px, 20px)',
    '!important',
    'margin: 10px',
    'div[foo="bar"]+a',
    'text <span data-x=https://a.test>div',
    'div[attr="a>b"]>span',
    '<div attr="a>b">span',
    '<div attr=\u{1f600}>span',
]

const cases = INPUTS.flatMap((line) => {
    const cursors = [0, ...Array.from(line).map((_, index, characters) => characters.slice(0, index + 1).join('').length)]
    return ['html', 'jsx', 'css'].flatMap((syntax) =>
        cursors.map((cursor) => {
            const value = extract(line, cursor, { type: syntax === 'css' ? 'stylesheet' : 'markup', lookAhead: syntax !== 'css' })
            const byte = (position) => new TextEncoder().encode(line.slice(0, position)).length
            const expected = value ? `Some((${literal(value.abbreviation)}, ${byte(value.location)}, ${byte(value.end)}))` : 'None'
            return `    (${literal(syntax)}, ${literal(line)}, ${byte(cursor)}, ${expected}),`
        }),
    )
})
const output = new URL('./emmet-extraction-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, &str, usize, Option<(&str, usize, usize)>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet oracle formatting failed')
console.info(JSON.stringify({ engine: 'emmet@2.4.11', inputs: INPUTS.length, cases: cases.length }))
