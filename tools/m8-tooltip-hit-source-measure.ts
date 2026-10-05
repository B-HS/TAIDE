import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const HALF = 0.5
const EMPTY_CORNER = 5
const TIP_OFFSET = 6
const CORNER_INSET = 0.25
const SIDES = ['top', 'bottom', 'left', 'right'] as const
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const output = process.argv[2]
const built = process.argv[3]
if (!output || !/^\/private\/tmp\/taide-tooltip-hit\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated hit directory')
if (!built || !/^\/private\/tmp\/taide-tooltip-motion\.[A-Za-z0-9]+\/built$/.test(built)) throw new Error('Reuse the measured motion build')
const browser = await chromium.launch({ executablePath: CHROME, headless: true })
try {
    const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
    await context.route('**/*', async (route) => {
        const url = new URL(route.request().url())
        if (url.origin !== ORIGIN) return route.abort()
        if (url.pathname === '/') {
            return route.fulfill({
                body: await Bun.file(resolve(built, 'experiments/native-tooltip-reference/motion.html')).text(),
                contentType: 'text/html',
            })
        }
        if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(url.pathname)) return route.abort()
        return route.fulfill({
            body: await Bun.file(resolve(built, url.pathname.slice(1))).text(),
            contentType: url.pathname.endsWith('.css') ? 'text/css' : 'text/javascript',
        })
    })
    const results = await Promise.all(
        SIDES.map(async (side) => {
            const page = await context.newPage()
            await page.goto(`${ORIGIN}/?side=${side}`)
            await page.waitForFunction(() => {
                const content = document.querySelector('[data-motion-content]')
                return content && getComputedStyle(content).opacity === '1' && getComputedStyle(content).visibility === 'visible'
            })
            const result = await page.locator('[data-motion-content]').evaluate(
                (content, constants) => {
                    const arrow = content.querySelector('svg')
                    const wrapper = arrow?.parentElement
                    if (!arrow || !wrapper) throw new Error('Missing actual Radix arrow')
                    const body = content.getBoundingClientRect()
                    const bounds = arrow.getBoundingClientRect()
                    const center = { x: bounds.x + bounds.width * constants.HALF, y: bounds.y + bounds.height * constants.HALF }
                    const direction = {
                        top: { x: 0, y: 1 },
                        bottom: { x: 0, y: -1 },
                        left: { x: 1, y: 0 },
                        right: { x: -1, y: 0 },
                    }[constants.side]
                    const points = [
                        { name: 'arrow-center', ...center },
                        { name: 'arrow-tip', x: center.x + direction.x * constants.TIP_OFFSET, y: center.y + direction.y * constants.TIP_OFFSET },
                        {
                            name: 'arrow-empty-corner',
                            x: center.x + (direction.x === -1 ? -constants.EMPTY_CORNER : constants.EMPTY_CORNER),
                            y: center.y + (direction.y === -1 ? -constants.EMPTY_CORNER : constants.EMPTY_CORNER),
                        },
                        { name: 'body-rounded-corner', x: body.x + constants.CORNER_INSET, y: body.y + constants.CORNER_INSET },
                    ]
                    return {
                        body: body.toJSON(),
                        arrow: bounds.toJSON(),
                        wrapper: wrapper.getBoundingClientRect().toJSON(),
                        samples: points.map((point) => {
                            const hit = document.elementFromPoint(point.x, point.y)
                            return {
                                ...point,
                                hitTag: hit?.tagName,
                                hitTooltip: hit !== null && content.contains(hit),
                                hitArrow: hit !== null && arrow.contains(hit),
                                hitWrapper: hit === wrapper,
                            }
                        }),
                    }
                },
                { HALF, EMPTY_CORNER, TIP_OFFSET, CORNER_INSET, side },
            )
            await page.close()
            return { side, ...result }
        }),
    )
    await Bun.write(resolve(output, 'hit.json'), JSON.stringify({ results }, null, 2))
    process.stdout.write(JSON.stringify({ results }, null, 2))
} finally {
    await browser.close()
}
