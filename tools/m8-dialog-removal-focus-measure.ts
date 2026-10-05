import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const built = '/private/tmp/taide-tooltip-escape.aWvZH4/built'
const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--use-mock-keychain'] })
try {
    const results = await Promise.all(
        ['Tab', 'Shift+Tab'].map(async (key) => {
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
                await page.goto(`${ORIGIN}/?mode=modal`)
                await page.locator('[data-open-dialog]').click()
                const dialog = page.locator('[data-slot="dialog-content"][data-state="open"]')
                await dialog.waitFor()
                await dialog.evaluate((element) => {
                    const first = document.createElement('input')
                    first.setAttribute('data-focus-first', '')
                    const removed = document.createElement('button')
                    removed.textContent = 'Synthetic removed control'
                    const last = document.createElement('button')
                    last.textContent = 'Synthetic last control'
                    last.setAttribute('data-focus-last', '')
                    element.append(first, removed, last)
                    removed.focus()
                    if (document.activeElement !== removed) throw new Error('Synthetic control did not acquire focus')
                    removed.remove()
                })
                await page.waitForFunction(() => document.activeElement?.getAttribute('data-slot') === 'dialog-content')
                const before = await dialog.evaluate((element) => document.activeElement === element)
                await page.keyboard.press(key)
                const after = await page.evaluate(() => ({
                    container: document.activeElement?.getAttribute('data-slot') === 'dialog-content',
                    first: document.activeElement?.hasAttribute('data-focus-first'),
                    last: document.activeElement?.hasAttribute('data-focus-last'),
                    tag: document.activeElement?.tagName,
                }))
                return { key, before, after }
            } finally {
                await context.close()
            }
        }),
    )
    process.stdout.write(JSON.stringify({ source: 'actual shared Dialog; synthetic DOM child removal; reused build', results }))
} finally {
    await browser.close()
}
