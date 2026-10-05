import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const FRAME_MS = 32
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const CLOCK_START = new Date('2026-10-04T00:00:00Z')
const CLOCK_PAUSED = new Date('2026-10-04T00:01:00Z')
const output = process.argv[2]
const built = process.argv[3]
if (!output || !/^\/private\/tmp\/taide-tooltip-keys\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated key directory')
if (!built || !/^\/private\/tmp\/taide-tooltip-events\.[A-Za-z0-9]+\/built$/.test(built)) throw new Error('Reuse the measured event build')
const browser = await chromium.launch({ executablePath: CHROME, headless: true })
try {
    const results = await Promise.all(
        ['focus-escape', 'escape-focus', 'initial-focus-escape'].map(async (order) => {
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
            await page.goto(`${ORIGIN}/?mode=plain`)
            await page.locator('[data-event-trigger="first"]').waitFor()
            await page.clock.runFor(FRAME_MS)
            if (order !== 'initial-focus-escape') await page.locator('[data-event-trigger="first"]').focus()
            const samples = []
            const steps = order === 'escape-focus' ? ['escape', 'focus'] : ['focus', 'escape']
            for (const step of steps) {
                if (step === 'focus') await page.locator('[data-event-trigger="second"]').focus()
                if (step === 'escape') await page.keyboard.press('Escape')
                await page.clock.runFor(FRAME_MS)
                samples.push({
                    step,
                    ...(await page.evaluate(() => ({
                        records: document.querySelector('[data-records]')?.textContent?.split('|'),
                        triggers: Array.from(document.querySelectorAll('[data-event-trigger]'), (element) => ({
                            id: element.getAttribute('data-event-trigger'),
                            state: element.getAttribute('data-state'),
                            describedBy: element.getAttribute('aria-describedby'),
                        })),
                    }))),
                })
            }
            await context.close()
            return { order, samples }
        }),
    )
    const result = { frameMs: FRAME_MS, cssPresencePaused: false, results }
    await Bun.write(resolve(output, 'keys.json'), JSON.stringify(result, null, 2))
    process.stdout.write(JSON.stringify(result, null, 2))
} finally {
    await browser.close()
}
