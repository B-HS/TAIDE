import { build } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const HALF = 0.5
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const output = process.argv[2]
const stylesheet = process.argv[3]
const option = process.argv[4]
if (!output || !/^\/private\/tmp\/taide-tooltip-escape\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated Escape directory')
if (!stylesheet || !/^\/private\/tmp\/taide-tooltip-source\.[A-Za-z0-9]+\/built\/assets\/[A-Za-z0-9_.-]+\.css$/.test(stylesheet))
    throw new Error('Reuse the measured source stylesheet')
if (option && option !== '--build-only' && option !== '--reuse-build') throw new Error('Unexpected Escape fixture option')
const cssSha256 = new Bun.CryptoHasher('sha256').update(await Bun.file(stylesheet).text()).digest('hex')
const repository = process.cwd()
const built = resolve(output, 'built')
if (option !== '--reuse-build') {
    await build({
        root: repository,
        configFile: false,
        envDir: false,
        publicDir: false,
        cacheDir: resolve(output, 'cache'),
        plugins: [react(), babel({ presets: [reactCompilerPreset()] })],
        resolve: {
            alias: [
                { find: '@shared/styles/global.css', replacement: stylesheet },
                { find: '@features', replacement: resolve(repository, 'src/features') },
                { find: '@shared', replacement: resolve(repository, 'src/shared') },
            ],
        },
        build: {
            outDir: built,
            emptyOutDir: false,
            rolldownOptions: { input: resolve(repository, 'experiments/native-tooltip-reference/escape.html') },
        },
    })
}
if (option === '--build-only') {
    process.stdout.write(JSON.stringify({ built, cssSha256 }))
    process.exit(0)
}
const browser = await chromium.launch({ executablePath: CHROME, headless: true })
try {
    const results = await Promise.all(
        ['valid', 'invalid', 'modal'].map(async (mode) => {
            const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
            try {
                await context.route('**/*', async (route) => {
                    const url = new URL(route.request().url())
                    if (url.origin !== ORIGIN) return route.abort()
                    if (url.pathname === '/')
                        return route.fulfill({
                            body: await Bun.file(resolve(built, 'experiments/native-tooltip-reference/escape.html')).text(),
                            contentType: 'text/html',
                        })
                    if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(url.pathname)) return route.abort()
                    return route.fulfill({
                        body: await Bun.file(resolve(built, url.pathname.slice(1))).text(),
                        contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript',
                    })
                })
                const page = await context.newPage()
                await page.goto(`${ORIGIN}/?mode=${mode}`)
                const trigger = page.locator('[data-escape-trigger]')
                await trigger.waitFor()
                const box = await trigger.boundingBox()
                if (!box) throw new Error('Missing Escape trigger bounds')
                await page.mouse.move(box.x + box.width * HALF, box.y + box.height * HALF)
                await page.locator('[data-escape-content]:not([data-state="closed"])').waitFor()
                if (mode === 'modal') {
                    await page.locator('[data-open-dialog]').evaluate((element) => {
                        if (!(element instanceof HTMLButtonElement)) throw new Error('Missing modal trigger')
                        element.click()
                    })
                    await page.locator('[data-slot="dialog-content"][data-state="open"]').waitFor()
                }
                const snapshot = async () =>
                    page.evaluate(() => ({
                        records: document.querySelector('[data-records]')?.textContent?.split('|'),
                        hasInput: document.querySelector('input') !== null,
                        inputFocused: document.activeElement instanceof HTMLInputElement,
                        normalState: document.querySelector('[data-escape-trigger]')?.getAttribute('data-state'),
                        normalDescription: document.querySelector('[data-escape-trigger]')?.getAttribute('aria-describedby'),
                        dialogState: document.querySelector('[data-dialog-state]')?.textContent,
                    }))
                const before = await snapshot()
                await page.keyboard.press('Escape')
                await page.waitForFunction(
                    (mode) =>
                        mode === 'modal'
                            ? document.querySelector('[data-dialog-state]')?.textContent === 'closed'
                            : document.querySelector('input') === null,
                    mode,
                )
                const after = await snapshot()
                return { mode, before, after }
            } finally {
                await context.close()
            }
        }),
    )
    const result = { cssSha256, actualDraftRow: true, virtualClock: false, cssPresencePaused: false, results }
    await Bun.write(resolve(output, 'escape.json'), JSON.stringify(result, null, 2))
    process.stdout.write(JSON.stringify(result, null, 2))
} finally {
    await browser.close()
}
