import { build } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const HALF = 0.5
const ORIGIN = 'https://taide-terminal-menu.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const output = process.argv[2]
const stylesheet = process.argv[3]
const option = process.argv[4]
const scenario = process.argv[5]
const scenarioKeys = Object.entries({
    '--mixed-search': [
        ['s', 'KeyS'],
        [' ', 'Space'],
    ],
    '--mixed-navigation': [
        ['ArrowDown', 'ArrowDown'],
        ['ArrowDown', 'ArrowDown'],
    ],
    '--owner-select-close': [
        ['End', 'End'],
        ['Enter', 'Enter'],
        ['t', 'KeyT'],
    ],
}).find(([name]) => name === scenario)?.[1]
if (!output || !/^\/private\/tmp\/taide-terminal-menu\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated terminal directory')
if (!stylesheet || !/^\/private\/tmp\/taide-tooltip-source\.[A-Za-z0-9]+\/built\/assets\/[A-Za-z0-9_.-]+\.css$/.test(stylesheet)) {
    throw new Error('Reuse the measured source stylesheet')
}
if (option && option !== '--build-only' && option !== '--reuse-build') throw new Error('Unexpected fixture option')
if (scenario && (!scenarioKeys || option !== '--reuse-build')) {
    throw new Error('Unexpected menu scenario')
}
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
                { find: '@shared', replacement: resolve(repository, 'src/shared') },
                { find: '@features', replacement: resolve(repository, 'src/features') },
            ],
        },
        build: {
            outDir: built,
            emptyOutDir: false,
            rolldownOptions: { input: resolve(repository, 'experiments/native-terminal-reference/index.html') },
        },
    })
}
if (option === '--build-only') {
    process.stdout.write(JSON.stringify({ built, cssSha256 }))
    process.exit(0)
}
const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--use-mock-keychain'] })
try {
    const results = []
    for (const mode of scenario ? ['plain'] : ['plain', 'sgr']) {
        const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
        try {
            await context.route('**/*', async (route) => {
                const url = new URL(route.request().url())
                if (url.origin !== ORIGIN) return route.abort()
                if (url.pathname === '/') {
                    return route.fulfill({
                        body: await Bun.file(resolve(built, 'experiments/native-terminal-reference/index.html')).text(),
                        contentType: 'text/html',
                    })
                }
                if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(url.pathname)) return route.abort()
                return route.fulfill({
                    body: await Bun.file(resolve(built, url.pathname.slice(1))).text(),
                    contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript',
                })
            })
            const page = await context.newPage()
            await page.goto(`${ORIGIN}/?mode=${mode}`)
            await page.locator('[data-ready]').filter({ hasText: 'ready' }).waitFor({ state: 'attached' })
            await page.locator('.xterm-helper-textarea').focus()
            await page.keyboard.type('con')
            const box = await page.locator('.xterm-screen').boundingBox()
            if (!box) throw new Error('Missing actual xterm bounds')
            const point = { x: box.x + box.width * HALF, y: box.y + box.height * HALF }
            await page.mouse.move(point.x, point.y)
            const snapshot = async (step: string) => ({
                step,
                ...(await page.evaluate(() => ({
                    records: document.querySelector('[data-records]')?.textContent?.trim().split('\n') ?? [],
                    hasMenu: document.querySelector('[role="menu"]') !== null,
                    activeTag: document.activeElement?.tagName.toLowerCase(),
                    activeRole: document.activeElement?.getAttribute('role'),
                    activeClass: document.activeElement?.className,
                    activeText: document.activeElement?.textContent,
                }))),
            })
            const before = await snapshot('before-secondary')
            await page.mouse.down({ button: 'right' })
            const pressed = await snapshot('secondary-down')
            await page.mouse.up({ button: 'right' })
            await page.getByRole('menu').waitFor()
            const released = await snapshot('secondary-up')
            if (scenario && scenarioKeys) {
                const ownerSelection = scenario === '--owner-select-close'
                if (!ownerSelection) await page.getByRole('menuitem', { name: 'Clear', exact: true }).focus()
                const focused = await snapshot(ownerSelection ? 'focused-owner' : 'focused-clear')
                await page.evaluate((keys) => {
                    for (const [key, code] of keys) {
                        for (const type of ['keydown', 'keyup']) {
                            document.activeElement?.dispatchEvent(new KeyboardEvent(type, { key, code, bubbles: true, cancelable: true }))
                        }
                    }
                }, scenarioKeys)
                if (ownerSelection) {
                    const selected = await snapshot('owner-end-enter-character')
                    if (!selected.records.includes('action:kill') || selected.records.includes('data:"t"')) {
                        throw new Error('Source owner selection or close input differs')
                    }
                    await page.getByRole('menu').waitFor({ state: 'detached' })
                    await page.locator('.xterm-helper-textarea').evaluate((element) => {
                        if (document.activeElement !== element) throw new Error('Selected menu did not restore terminal focus')
                    })
                    await page.keyboard.type('tinue')
                    results.push({
                        mode,
                        point,
                        syntheticSameTask: true,
                        samples: [before, pressed, released, focused, selected, await snapshot('selected-detached-input')],
                    })
                    continue
                }
                await page.waitForFunction(() => document.activeElement?.textContent === 'Split')
                const searched = await snapshot(scenario.slice('--'.length))
                if (!searched.hasMenu || searched.records.includes('action:clear')) throw new Error('Source mixed input selected Clear')
                results.push({ mode, point, syntheticSameTask: true, samples: [before, pressed, released, focused, searched] })
                continue
            }
            await page.keyboard.press('Escape')
            await page.getByRole('menu').waitFor({ state: 'detached' })
            await page.locator('.xterm-helper-textarea').evaluate((element) => {
                if (document.activeElement !== element) throw new Error('Original terminal focus did not return')
            })
            await page.keyboard.type('tinue')
            results.push({ mode, point, samples: [before, pressed, released, await snapshot('escape-return-input')] })
        } finally {
            await context.close()
        }
    }
    const result = { cssSha256, browserVersion: browser.version(), platform: process.platform, results }
    await Bun.write(resolve(output, scenario ? `menu-${scenario.slice('--'.length)}.json` : 'menu.json'), JSON.stringify(result, null, 2))
    process.stdout.write(JSON.stringify(result, null, 2))
} finally {
    await browser.close()
}
