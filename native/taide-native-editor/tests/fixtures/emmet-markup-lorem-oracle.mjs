import { expandAbbreviation } from '../../../../node_modules/emmet-monaco-es/src/emmetHelper.ts'
import { fileURLToPath } from 'node:url'

import { rustLiteral } from './emmet-rust-literal.mjs'

const MULTIPLIER = 1664525
const INCREMENT = 1013904223
const RANDOM_SCALE = 2 ** 32
const SEEDS = [1, 42, 987654321]
const MAX_REPEATS = 7
const source = await Bun.file(new URL('../../../../node_modules/emmet/dist/emmet.es.js', import.meta.url)).text()
const vocabularies = ['latin', 'ru', 'sp'].map((name) => {
    const text = source.match(new RegExp(`var ${name} = (\\{[\\s\\S]*?\\n\\});`))?.[1]
    if (!text) throw new Error(`Missing vocabulary ${name}`)
    const value = JSON.parse(text)
    if (!Array.isArray(value.common) || !Array.isArray(value.words) || [...value.common, ...value.words].some((word) => typeof word !== 'string')) {
        throw new Error(`Invalid vocabulary ${name}`)
    }
    return { name, ...value }
})
const defaults = vocabularies
    .flatMap(({ name, common, words }) => [
        `pub const ${name.toUpperCase()}_COMMON: &[&str] = &[${common.map(rustLiteral).join(', ')}];`,
        `pub const ${name.toUpperCase()}_WORDS: &[&str] = &[${words.map(rustLiteral).join(', ')}];`,
    ])
    .join('\n')
const defaultsPath = new URL('../../src/emmet-markup-lorem-defaults.rs', import.meta.url)
await Bun.write(defaultsPath, defaults + '\n')
const stems = [
    'lorem',
    'lorem0',
    'lorem1',
    'lorem2',
    'lorem8',
    'lorem9',
    'lorem30',
    'lorem100',
    'lorem5-20',
    'lorem20-5',
    'lorem3-',
    'loremru12',
    'loremsp40',
    'loremfr12',
    'LOREMRU12',
    'loremru1',
    'loremsp1',
    'lorem0008',
    'lorem-8',
    'lorem-0',
    'lorem*3',
    'lorem5*3',
    'p>lorem5*3',
    'p*3>lorem12',
    'ul>lorem8*2',
    'ul*2>li>lorem10',
    '(h2{title}+p>lorem12)*2',
    'div>lorem5>span{end}',
    'p>lorem5.class#id[title=ignored]',
    'div.block>p.-element>lorem12',
    'lorem5{ignored}',
    'loremx5+span',
    'lorem5x',
    'lorem1.5',
    'lorem2-4-6',
    'p>loremru12*2',
    'table>loremsp9*2',
]
const suffixes = ['', '|bem|c']
const originalRandom = Math.random
const cases = []
try {
    for (const syntax of ['html', 'jsx']) {
        for (const seed of SEEDS) {
            for (const stem of stems) {
                for (const suffix of suffixes) {
                    let state = seed
                    Math.random = () => {
                        state = (Math.imul(state, MULTIPLIER) + INCREMENT) >>> 0
                        return state / RANDOM_SCALE
                    }
                    let expected = 'None'
                    try {
                        const output = expandAbbreviation(stem, {
                            type: 'markup',
                            syntax,
                            maxRepeat: MAX_REPEATS,
                            options: {
                                'output.formatSkip': ['html'],
                                'output.formatForce': ['body'],
                                'output.field': (index, placeholder) => `\${${index}${placeholder ? ':' + placeholder : ''}}`,
                                'output.inlineBreak': 0,
                                'output.compactBoolean': false,
                                'output.reverseAttributes': false,
                                'markup.href': true,
                                'jsx.enabled': syntax === 'jsx',
                                'output.format': true,
                                'output.selfClosingStyle': 'html',
                                'bem.enabled': suffix.length > 0,
                                'comment.enabled': suffix.length > 0,
                                'comment.trigger': ['id', 'class'],
                                'comment.before': '',
                                'comment.after': '\n<!-- /[#ID][.CLASS] -->',
                            },
                        })
                        expected = `Some(${rustLiteral(output)})`
                    } catch {}
                    cases.push(`    (${rustLiteral(syntax)}, ${seed}, ${rustLiteral(stem + suffix)}, ${expected}),`)
                }
            }
        }
    }
} finally {
    Math.random = originalRandom
}
const output = new URL('./emmet-markup-lorem-oracle.rs', import.meta.url)
await Bun.write(output, `const CASES: &[(&str, u32, &str, Option<&str>)] = &[\n${cases.join('\n')}\n];\n`)
const formatted = Bun.spawnSync(['rustfmt', '--edition', '2024', fileURLToPath(defaultsPath), fileURLToPath(output)])
if (formatted.exitCode !== 0) throw new Error('Emmet lorem oracle formatting failed')
console.info(
    JSON.stringify({ engine: 'emmet@2.4.11', vocabularies: vocabularies.length, stems: stems.length, seeds: SEEDS.length, cases: cases.length }),
)
