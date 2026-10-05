import { build } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const FRAME_MS = 32
const SKIP_ELAPSED_MS = 350
const DELAY_MS = 400
const HALF = 0.5
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const CLOCK_START = new Date('2026-10-04T00:00:00Z')
const CLOCK_PAUSED = new Date('2026-10-04T00:01:00Z')
const output = process.argv[2]
const stylesheet = process.argv[3]
const option = process.argv[4]
if (!output || !/^\/private\/tmp\/taide-tooltip-events\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated event directory')
if (!stylesheet || !/^\/private\/tmp\/taide-tooltip-source\.[A-Za-z0-9]+\/built\/assets\/[A-Za-z0-9_.-]+\.css$/.test(stylesheet)) {
    throw new Error('Reuse the measured source stylesheet')
}
if (option && option !== '--build-only' && option !== '--reuse-build') throw new Error('Unexpected event fixture option')
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
            ],
        },
        build: {
            outDir: built,
            emptyOutDir: false,
            rolldownOptions: { input: resolve(repository, 'experiments/native-tooltip-reference/events.html') },
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
        ['plain', 'controlled'].map(async (mode) => {
            const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
            await context.route('**/*', async (route) => {
                const url = new URL(route.request().url())
                if (url.origin !== ORIGIN) return route.abort()
                if (url.pathname === '/')
                    return route.fulfill({
                        body: await Bun.file(resolve(built, 'experiments/native-tooltip-reference/events.html')).text(),
                        contentType: 'text/html',
                    })
                if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(url.pathname)) return route.abort()
                return route.fulfill({
                    body: await Bun.file(resolve(built, url.pathname.slice(1))).text(),
                    contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript',
                })
            })
            const page = await context.newPage()
            await page.clock.install({ time: CLOCK_START })
            await page.clock.pauseAt(CLOCK_PAUSED)
            await page.goto(`${ORIGIN}/?mode=${mode}`)
            await page.locator('[data-event-trigger="first"]').waitFor()
            await page.clock.runFor(FRAME_MS)
            const snapshot = async (step: string) => ({
                step,
                ...(await page.evaluate(() => {
                    for (const element of document.querySelectorAll('[data-event-content]')) {
                        for (const animation of element.getAnimations()) animation.pause()
                    }
                    const records = document.querySelector('[data-records]')?.textContent
                    if (records === null || records === undefined) throw new Error('Missing event records')
                    return {
                        records: records ? records.split('|') : [],
                        active:
                            document.activeElement?.getAttribute('data-event-trigger') ?? document.activeElement?.getAttribute('data-event-input'),
                        hasInput: document.querySelector('[data-event-input]') !== null,
                        contents: Array.from(document.querySelectorAll('[data-event-content]'), (element) => ({
                            id: element.getAttribute('data-event-content'),
                            state: element.getAttribute('data-state'),
                        })),
                    }
                })),
            })
            const initial = await snapshot('initial')
            const focused = []
            for (const id of ['first', 'second', 'first']) {
                await page.locator(`[data-event-trigger="${id}"]`).focus()
                await page.clock.runFor(FRAME_MS)
                focused.push(await snapshot(`focus-${id}`))
            }
            await page.clock.runFor(SKIP_ELAPSED_MS)
            const box = await page.locator('[data-event-trigger="second"]').boundingBox()
            if (!box) throw new Error('Missing second trigger bounds')
            await page.mouse.move(box.x + box.width * HALF, box.y + box.height * HALF)
            await page.clock.runFor(FRAME_MS)
            const hovered = await snapshot('hover-second-before-delay')
            await page.clock.runFor(DELAY_MS)
            const delayed = await snapshot('hover-second-after-delay')
            await context.close()
            return { mode, samples: [initial, ...focused, hovered, delayed] }
        }),
    )
    const result = { cssSha256, frameMs: FRAME_MS, skipElapsedMs: SKIP_ELAPSED_MS, cssPresencePaused: true, results }
    await Bun.write(resolve(output, 'events.json'), JSON.stringify(result, null, 2))
    process.stdout.write(JSON.stringify(result, null, 2))
} finally {
    await browser.close()
}
