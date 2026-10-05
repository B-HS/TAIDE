import { build } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import { chromium } from '@playwright/test'
import { resolve } from 'node:path'

const WIDTH = 640
const HEIGHT = 480
const SAMPLE_COUNT = 8
const ORIGIN = 'https://taide-tooltip.invalid'
const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const output = process.argv[2]
if (!output || !/^\/private\/tmp\/taide-tooltip-source\.[A-Za-z0-9]+$/.test(output)) {
    throw new Error('Use a dedicated mktemp taide-tooltip-source directory')
}
const repository = process.cwd()
const built = resolve(output, 'built')
const shouldReuseBuild = process.argv[3] === '--reuse-build'
if (process.argv[3] && !shouldReuseBuild) throw new Error('Unexpected fixture option')
if (!shouldReuseBuild)
    await build({
        root: repository,
        configFile: false,
        envDir: false,
        publicDir: false,
        cacheDir: resolve(output, 'cache'),
        plugins: [react(), babel({ presets: [reactCompilerPreset()] }), tailwindcss()],
        resolve: { alias: { '@shared': resolve(repository, 'src/shared') } },
        build: {
            outDir: built,
            emptyOutDir: false,
            rolldownOptions: { input: resolve(repository, 'experiments/native-tooltip-reference/index.html') },
        },
    })
const browser = await chromium.launch({ executablePath: CHROME, headless: true })
try {
    const context = await browser.newContext({ viewport: { width: WIDTH, height: HEIGHT }, serviceWorkers: 'block' })
    await context.route('**/*', async (route) => {
        const url = new URL(route.request().url())
        if (url.origin !== ORIGIN) return route.abort()
        const path = url.pathname
        if (path === '/') {
            return route.fulfill({
                body: await Bun.file(resolve(built, 'experiments/native-tooltip-reference/index.html')).text(),
                contentType: 'text/html',
            })
        }
        if (!/^\/assets\/[A-Za-z0-9_.-]+\.(css|js)$/.test(path)) return route.abort()
        return route.fulfill({
            body: await Bun.file(resolve(built, path.slice(1))).text(),
            contentType: path.endsWith('.css') ? 'text/css' : 'text/javascript',
        })
    })
    const page = await context.newPage()
    await page.goto(ORIGIN)
    await page.waitForFunction((count) => {
        const contents = Array.from(document.querySelectorAll('[data-sample-content]'))
        const flippedTop = document.querySelector('[data-sample-content="flip-top"]')
        const flippedBottom = document.querySelector('[data-sample-content="flip-bottom"]')
        return (
            flippedTop?.getAttribute('data-side') === 'bottom' &&
            flippedBottom?.getAttribute('data-side') === 'top' &&
            contents.length === count &&
            contents.every(
                (content) =>
                    content.getAnimations().every((animation) => animation.playState === 'finished') &&
                    getComputedStyle(content).opacity === '1' &&
                    getComputedStyle(content).visibility === 'visible' &&
                    content.getBoundingClientRect().x > -window.innerWidth,
            )
        )
    }, SAMPLE_COUNT)
    const samples = await page.locator('[data-sample-content]').evaluateAll((contents) =>
        contents.map((content) => {
            const arrow = content.querySelector('svg')
            const wrapper = arrow?.parentElement
            const id = content.getAttribute('data-sample-content')
            const trigger = document.querySelector(`[data-sample="${id}"]`)
            if (!arrow || !wrapper || !trigger) throw new Error('Incomplete synthetic tooltip DOM')
            const style = getComputedStyle(content)
            const arrowStyle = getComputedStyle(arrow)
            const wrapperStyle = getComputedStyle(wrapper)
            return {
                id,
                side: content.getAttribute('data-side'),
                align: content.getAttribute('data-align'),
                trigger: trigger.getBoundingClientRect().toJSON(),
                content: content.getBoundingClientRect().toJSON(),
                arrow: arrow.getBoundingClientRect().toJSON(),
                wrapper: wrapper.getBoundingClientRect().toJSON(),
                font: { size: style.fontSize, family: style.fontFamily, lineHeight: style.lineHeight },
                animation: { duration: style.animationDuration, timing: style.animationTimingFunction, origin: style.transformOrigin },
                arrowStyle: {
                    width: arrowStyle.width,
                    height: arrowStyle.height,
                    translate: arrowStyle.translate,
                    rotate: arrowStyle.rotate,
                    radius: arrowStyle.borderRadius,
                    overflow: arrowStyle.overflow,
                    background: arrowStyle.backgroundColor,
                    fill: arrowStyle.fill,
                },
                wrapperStyle: {
                    transform: wrapperStyle.transform,
                    origin: wrapperStyle.transformOrigin,
                    left: wrapperStyle.left,
                    top: wrapperStyle.top,
                    right: wrapperStyle.right,
                    bottom: wrapperStyle.bottom,
                    visibility: wrapperStyle.visibility,
                },
            }
        }),
    )
    await page.screenshot({ path: resolve(output, 'source.png') })
    const report = { viewport: { width: WIDTH, height: HEIGHT }, samples }
    await Bun.write(resolve(output, 'source.json'), JSON.stringify(report, null, 2))
    process.stdout.write(
        JSON.stringify(
            samples.map((sample) => ({
                id: sample.id,
                side: sample.side,
                content: sample.content,
                arrow: sample.arrow,
                lineHeight: sample.font.lineHeight,
                arrowStyle: sample.arrowStyle,
                wrapperStyle: sample.wrapperStyle,
            })),
            null,
            2,
        ),
    )
} finally {
    await browser.close()
}
