import { rustLiteral as literal } from './emmet-rust-literal.mjs'
import abbreviation from '@emmetio/abbreviation'
import { fileURLToPath } from 'node:url'

import { INPUTS } from './emmet-markup-inputs.mjs'
import { canonicalNodes } from './emmet-markup-canonical.mjs'

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

const cases = ['html', 'jsx'].flatMap((syntax) =>
    [false, true].flatMap((withVariables) =>
        CASE_INPUTS.map((input) => {
            let expected = 'None'
            try {
                const tree = abbreviation(input, { jsx: syntax === 'jsx', maxRepeat: MAX_REPEATS, variables: withVariables ? VARIABLES : {} })
                expected = `Some(${literal(canonicalNodes(tree.children))})`
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
