import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const BUILT = '/private/tmp/taide-tooltip-events.lDAI8X/built'
const DEFAULT_CASES = ['space', 'cancel-down', 'cancel-up', 'blur', 'enter'] as const
const ESCAPE_CASES = ['escape', 'capture-escape'] as const
const option = process.argv[2]
if (option !== undefined && option !== '--escape-only') throw new Error('Unexpected button source option')
const CASES = option === '--escape-only' ? ESCAPE_CASES : DEFAULT_CASES
const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--use-mock-keychain'] })
try {
    const results = await Promise.all(
        CASES.map(async (mode) => {
            const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
            try {
                await context.route('**/*', async (route) => {
                    const url = new URL(route.request().url())
                    if (url.origin !== ORIGIN) return route.abort()
                    if (url.pathname === '/')
                        return route.fulfill({
                            body: await Bun.file(resolve(BUILT, 'experiments/native-tooltip-reference/events.html')).text(),
                            contentType: 'text/html',
                        })
                    if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(url.pathname)) return route.abort()
                    return route.fulfill({
                        body: await Bun.file(resolve(BUILT, url.pathname.slice(1))).text(),
                        contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript',
                    })
                })
                const page = await context.newPage()
                await page.goto(ORIGIN)
                const first = page.locator('[data-event-trigger=first]')
                await first.evaluate((element, cancel) => {
                    if (cancel === 'capture-escape') {
                        window.addEventListener(
                            'keydown',
                            (event) => {
                                event.preventDefault()
                                event.stopPropagation()
                            },
                            true,
                        )
                    }
                    for (const type of ['keydown', 'keyup', 'click', 'blur']) {
                        element.addEventListener(type, (event) => {
                            if ((cancel === 'cancel-down' && event.type === 'keydown') || (cancel === 'cancel-up' && event.type === 'keyup'))
                                event.preventDefault()
                            const existing = element.getAttribute('data-key-events') ?? ''
                            element.setAttribute('data-key-events', `${existing}|${event.type}:${event.defaultPrevented}`)
                            if (event.type === 'click')
                                element.setAttribute('data-click-count', String(Number(element.getAttribute('data-click-count') ?? '0') + 1))
                        })
                    }
                }, mode)
                await first.focus()
                const sample = async () => ({
                    events: await first.getAttribute('data-key-events'),
                    clicks: Number((await first.getAttribute('data-click-count')) ?? '0'),
                    state: await first.getAttribute('data-state'),
                    focused: await first.evaluate((element) => document.activeElement === element),
                })
                const KEY = {
                    space: 'Space',
                    'cancel-down': 'Space',
                    'cancel-up': 'Space',
                    blur: 'Space',
                    enter: 'Enter',
                    escape: 'Escape',
                    'capture-escape': 'Escape',
                }
                const key = KEY[mode]
                await page.keyboard.down(key)
                const down = await sample()
                await page.keyboard.down(key)
                const repeat = await sample()
                if (mode === 'blur') await page.locator('[data-event-trigger=second]').focus()
                await page.keyboard.up(key)
                const up = await sample()
                return { mode, down, repeat, up }
            } finally {
                await context.close()
            }
        }),
    )
    process.stdout.write(JSON.stringify({ source: 'actual shared Tooltip; native HTML button; reused events build', results }))
} finally {
    await browser.close()
}
