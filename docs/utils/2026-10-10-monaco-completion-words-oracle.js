import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { DEFAULT_WORD_REGEXP } from '../../node_modules/monaco-editor/esm/vs/editor/common/core/wordHelper.js'
import { EditorWorker } from '../../node_modules/monaco-editor/esm/vs/editor/common/services/editorWebWorker.js'

const ROOT = resolve(import.meta.dir, '../..')
const configurations = JSON.parse(readFileSync(resolve(ROOT, 'native/taide-native-syntax/language-configurations/monaco.json'), 'utf8'))
const NUMERIC_TOKENS = [
    '',
    '0',
    '01',
    '1',
    '.1',
    '1.',
    '-.1',
    '+.1',
    '1e3',
    '1e+3',
    '1E-3',
    '1e',
    '1e+',
    '1e-999999',
    '1e999999',
    'Infinity',
    '-Infinity',
    '+Infinity',
    'infinity',
    'inf',
    'NaN',
    '0xFF',
    '0Xff',
    '0x',
    '0b10',
    '0B10',
    '0b2',
    '0o77',
    '0O77',
    '0o8',
    '-0x1',
    '+0b1',
    '1_000',
    '1n',
    '１２３',
    '١٢٣',
    '--1',
    '+-1',
    '1.2.3',
    '1 2',
]
const NUMERIC_PREFIXES = ['', '+', '-', ' ', '\t', '\n', '\u00a0', '\u0085', '\ufeff', '\u2000', '\u180e', '\u2028', '\u3000']
const NUMERIC_SUFFIXES = ['', ' ', '\r\n', '\u0085', '\ufeff', '\u202f', 'f', 'e0', '.0']
const numeric = NUMERIC_TOKENS.flatMap((token) =>
    NUMERIC_PREFIXES.flatMap((prefix) =>
        NUMERIC_SUFFIXES.map((suffix) => {
            const text = `${prefix}${token}${suffix}`
            return [text, !Number.isNaN(Number(text))]
        }),
    ),
)
const CORPUS = [
    'console Console const concat console 0 01 0xff 0b11 0o77 1e3 1e999999 Infinity NaN infinity 1_000 1n',
    '한글 한글값 Σίσυφος İstanbul cafe\u0301 \u{1f600}word fooBar _private $value -1.25 10.4px .3e5 １２３ ١٢٣',
    'use namespace std::string alpha.beta foo-bar email@example.com path/to/file #include <vector> --name :symbol',
    'repeated repeated 1.0e+3 \u0085word \ufeffword \u2000word',
]
const OTHER_DOCUMENT = 'otherDocument console 다른문서 fooBar NaN 1000 trailing_symbol'
const LEADING_WORDS = ['con', 'console', '한글값', '', 'Infinity']
const languageIds = [...configurations.languages.map((language) => language.languageId), 'synthetic-plugin-language']
const words = []
for (const languageId of languageIds) {
    const configuration = configurations.languages.find((language) => language.languageId === languageId)?.configuration
    const pattern = configuration?.wordPattern
    const source = typeof pattern === 'string' ? pattern : (pattern?.source ?? DEFAULT_WORD_REGEXP.source)
    const flags = typeof pattern === 'string' ? 'g' : (pattern?.flags ?? DEFAULT_WORD_REGEXP.flags)
    for (const leading of LEADING_WORDS) {
        const documents = [`${leading}\n${CORPUS.join('\n')}`, OTHER_DOCUMENT]
        const worker = new EditorWorker()
        const urls = documents.map((text, index) => {
            const url = `inmemory://completion/${index}`
            worker.$acceptNewModel({ url, lines: text.split('\n'), EOL: '\n', versionId: 1 })
            return url
        })
        const result = await worker.$textualSuggest(urls, leading, source, flags)
        words.push({ languageId, leading, documents, expected: result.words })
        worker.dispose()
    }
}
const fixtureDirectory = resolve(ROOT, 'native/taide-native-app/src/fixtures')
mkdirSync(fixtureDirectory, { recursive: true })
writeFileSync(resolve(fixtureDirectory, 'completion-words-reference.json'), `${JSON.stringify({ numeric, words })}\n`)
process.stdout.write(`Monaco completion words: ${words.length} cases; JavaScript Number: ${numeric.length} cases\n`)
process.exit(0)
