import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

const ROOT = resolve(import.meta.dir, '../..')
const source = readFileSync(resolve(ROOT, 'node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations/browser/linesOperations.js'), 'utf8')
const classes = source.slice(source.indexOf('class UpperCaseAction'), source.indexOf('registerEditorAction(CopyLinesUpAction)'))
const inputs = [
    '',
    'helloWorld',
    'HTTPServer',
    'XMLHttpRequest',
    'a_b_c',
    '_hello-world_',
    '  hello world  ',
    'HELLO_WORLD',
    'foo.bar BAZ',
    'a\nb_c',
    'a\r\nb-c',
    "DON’T STOP, don't rock 'N' ROLL",
    "a 'hello' d’artagnan",
    'ß Straße İΣ ΟΣ',
    '１２Three ĀĭÇafe',
    '한글中文 fooBar',
    'ǅABC',
    '𐐀𐐨_𐐨ABC',
    '\u{0345}alpha',
    '99Bottles',
    'a\u{a0}b',
    'A\nABC',
    'a__b___c',
    'aa_bb_cc',
    'a_\u{1f4a1}_b',
    'x\tY',
]
const expression = `class AbstractCaseAction {}\n${classes}\nJSON.stringify(${JSON.stringify(inputs)}.map((input) => ({ input, outputs: [UpperCaseAction, LowerCaseAction, TitleCaseAction, SnakeCaseAction, CamelCaseAction, PascalCaseAction, KebabCaseAction].map((Action) => Action.prototype._modifyText.call(Object.create(Action.prototype), input, '')) })), null, 2)`
const result = eval(expression)
writeFileSync(resolve(ROOT, 'native/taide-native-syntax/src/monaco-case-oracle.json'), `${result}\n`)
