import { createHash } from 'node:crypto'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { runInNewContext } from 'node:vm'
import { chromium } from '@playwright/test'

const ROOT = resolve(import.meta.dir, '../..')
const SOURCE = 'node_modules/monaco-editor/esm/vs/editor/contrib/suggest/browser/suggestWidgetRenderer.js'
const FIXTURE = resolve(ROOT, 'native/taide-native-app/src/fixtures/completion-colors-reference.json')
const BEGIN = 'const _completionItemColor = '
const END = 'let ItemRenderer = '
const COLOR_KIND = 19
const CHECK_OPTION = '--check'
const inputArguments = process.argv.slice(2)
if (inputArguments.length > 1 || inputArguments.some((argument) => argument !== CHECK_OPTION)) throw new Error('Only --check is supported')
const source = readFileSync(resolve(ROOT, SOURCE), 'utf8')
const begin = source.indexOf(BEGIN)
const end = source.indexOf(END, begin)
if (begin < 0 || end <= begin) throw new Error('Original ColorExtractor boundary differs')
const extractor = runInNewContext(`${source.slice(begin, end)}\n_completionItemColor`)
const COLORS = [
    '#abc',
    '#ABCDEF',
    '#aBcDeF',
    '#fff',
    '#000',
    '#123456',
    'rgb(0, 0, 0)',
    'rgb(255,128,1)',
    'rgb(100%, 50%, 0%)',
    'rgb(999, 256, 257)',
    'RGB(1, 2, 3)',
    'rgba(255, 0, 0, .5)',
    'rgba(255,0,0,0.0)',
    'rgba(0,0,255,1)',
    'rgba(20%,40%,60%,0.3)',
    'rgba(255,0,128,0.01)',
    'rgba(255,0,128,0.99)',
    'hsl(0, 100%, 50%)',
    'hsl(120,100%,50%)',
    'hsl(240,100%,50%)',
    'hsl(360,100%,50%)',
    'hsl(999,100%,50%)',
    'hsl(30, 50%, 25%)',
    'hsla(270,100%,50%,0.25)',
    'HSLA(0,100%,50%,.5)',
    'hsl(1,0%,0%)',
    'hsl(30,999%,999%)',
    'hsl(15,999%,50%)',
    'hsl(15,999%,25%)',
    'hsl(15,50%,999%)',
    'hsl(0%,100%,50%)',
    'hsl(0,100,50)',
    'rgb(1,2%,3)',
    '#abcd',
    '#abcdefgh',
    '#11223344',
    'red',
    'rgb(1 2 3)',
    'rgba(1,2,3,0)',
    'rgb(-1,0,0)',
    'rgb(1.5,2,3)',
    '#abc\n',
    '#abc ',
    ' rgb(1,2,3)',
]
const cases = [
    ...COLORS.flatMap((label) => [
        { label, kind: COLOR_KIND },
        { label: 'shade', detail: label, kind: COLOR_KIND },
        { label: 'shade', documentation: `${label} color`, kind: COLOR_KIND },
        { label: 'shade', documentation: `color ${label}`, kind: COLOR_KIND },
        { label: 'shade', documentation: { value: `color ${label}` }, kind: COLOR_KIND },
        { label: 'shade', documentation: `color ${label} middle`, kind: COLOR_KIND },
    ]),
    { label: '#f00', detail: '#0f0', documentation: '#00f', kind: COLOR_KIND },
    { label: 'shade', detail: '#0f0', documentation: '#00f', kind: COLOR_KIND },
    { label: 'shade', documentation: 'middle #f00 then #0f0', kind: COLOR_KIND },
    { label: 'shade', documentation: '#f00 then #0f0', kind: COLOR_KIND },
    { label: 'shade', documentation: '이름 #0f0', kind: COLOR_KIND },
    { label: '#fff', kind: 0 },
    { label: '#fff', kind: 20 },
    { label: '#fff', kind: 23 },
]
const extracted = cases.map((completion) => {
    const output = []
    const found = completion.kind === COLOR_KIND && extractor.extract({ textLabel: completion.label, completion }, output)
    return { completion, extracted: found ? output[0] : null }
})
if (inputArguments[0] === CHECK_OPTION) {
    const reference = JSON.parse(readFileSync(FIXTURE, 'utf8'))
    if (reference.sha256 !== createHash('sha256').update(source).digest('hex')) throw new Error('Original renderer hash differs')
    const stored = reference.cases.map(({ completion, extracted }) => ({ completion, extracted }))
    if (JSON.stringify(stored) !== JSON.stringify(extracted)) throw new Error('Original extraction reference differs')
    console.log(JSON.stringify({ fixture: FIXTURE, cases: extracted.length, result: 'unchanged' }))
    process.exit(0)
}
const browser = await chromium.launch({ channel: 'chrome', headless: true })
try {
    const page = await browser.newPage()
    const values = await page.evaluate(
        (entries) =>
            entries.map((entry) => {
                const element = document.createElement('span')
                element.style.backgroundColor = entry.extracted ?? ''
                const css = element.style.backgroundColor
                return { ...entry, css: css || null }
            }),
        extracted,
    )
    mkdirSync(dirname(FIXTURE), { recursive: true })
    writeFileSync(
        FIXTURE,
        `${JSON.stringify({ source: SOURCE, sha256: createHash('sha256').update(source).digest('hex'), browser: browser.version(), cases: values }, null, 4)}\n`,
    )
    console.log(JSON.stringify({ fixture: FIXTURE, cases: values.length, browser: browser.version() }))
} finally {
    await browser.close()
}
