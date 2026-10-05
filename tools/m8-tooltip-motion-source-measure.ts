import { build } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const TIMES = [0, 30, 75, 149]
const SIDES = ['top', 'bottom', 'left', 'right'] as const
const PHASES = ['closed', 'open'] as const
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const output = process.argv[2]
const stylesheet = process.argv[3]
if (!output || !/^\/private\/tmp\/taide-tooltip-motion\.[A-Za-z0-9]+$/.test(output)) throw new Error('Use a dedicated motion directory')
if (!stylesheet || !/^\/private\/tmp\/taide-tooltip-source\.[A-Za-z0-9]+\/built\/assets\/[A-Za-z0-9_.-]+\.css$/.test(stylesheet)) {
    throw new Error('Use the previously measured source stylesheet')
}
const css = await Bun.file(stylesheet).text()
const cssSha256 = new Bun.CryptoHasher('sha256').update(css).digest('hex')
const repository = process.cwd()
const built = resolve(output, 'built')
const shouldReuseBuild = process.argv[4] === '--reuse-build'
if (process.argv[4] && !shouldReuseBuild) throw new Error('Unexpected fixture option')
if (!shouldReuseBuild)
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
            rolldownOptions: { input: resolve(repository, 'experiments/native-tooltip-reference/motion.html') },
        },
    })
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
    const results = []
    for (const side of SIDES) {
        const page = await context.newPage()
        await page.goto(`${ORIGIN}/?side=${side}`)
        await page.waitForFunction(() => {
            const content = document.querySelector('[data-motion-content]')
            return content && getComputedStyle(content).opacity === '1' && getComputedStyle(content).visibility === 'visible'
        })
        const base = await page.locator('[data-motion-content]').evaluate((element) => element.getBoundingClientRect().toJSON())
        for (const phase of PHASES) {
            await page.locator(phase === 'closed' ? '[data-close]' : '[data-open]').evaluate((element) => {
                if (!(element instanceof HTMLButtonElement)) throw new Error('Missing synthetic button')
                element.click()
            })
            await page.waitForFunction((phase) => {
                const content = document.querySelector('[data-motion-content]')
                const animation = content?.getAnimations()[0]
                const state = content?.getAttribute('data-state')
                const expected = phase === 'closed' ? state === 'closed' : state === 'instant-open' || state === 'delayed-open'
                if (!expected || !(animation instanceof CSSAnimation) || animation.animationName !== (phase === 'closed' ? 'exit' : 'enter'))
                    return false
                animation.pause()
                return true
            }, phase)
            const measured = await page.locator('[data-motion-content]').evaluate((content, times) => {
                const animation = content.getAnimations()[0]
                if (!animation || !(animation.effect instanceof KeyframeEffect)) throw new Error('Missing CSS motion')
                const keyframes = animation.effect.getKeyframes()
                const duration = animation.effect.getComputedTiming().duration
                const samples = times.map((time) => {
                    animation.currentTime = time
                    const style = getComputedStyle(content)
                    const matrix = new DOMMatrixReadOnly(style.transform)
                    const trigger = document.querySelector('[data-trigger]')
                    const arrow = content.querySelector('svg')
                    return {
                        time,
                        opacity: Number(style.opacity),
                        scale: matrix.a,
                        translateX: matrix.e,
                        translateY: matrix.f,
                        origin: style.transformOrigin.split(' ').map((value) => Number.parseFloat(value)),
                        content: content.getBoundingClientRect().toJSON(),
                        arrow: arrow?.getBoundingClientRect().toJSON(),
                        role: content.getAttribute('role'),
                        state: content.getAttribute('data-state'),
                        described: trigger?.hasAttribute('aria-describedby'),
                    }
                })
                animation.finish()
                return { duration, keyframes, samples }
            }, TIMES)
            if (phase === 'closed') await page.waitForFunction(() => document.querySelector('[data-motion-content]') === null)
            results.push({ side, phase, base, removedAfterFinish: phase === 'closed', ...measured })
        }
        await page.close()
    }
    await Bun.write(resolve(output, 'motion.json'), JSON.stringify({ cssSha256, results }, null, 2))
    process.stdout.write(
        JSON.stringify(
            results.map(({ side, phase, duration, samples, removedAfterFinish }) => ({ side, phase, duration, removedAfterFinish, samples })),
            null,
            2,
        ),
    )
} finally {
    await browser.close()
}
