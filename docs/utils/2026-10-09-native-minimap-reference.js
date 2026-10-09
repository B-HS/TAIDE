import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath, pathToFileURL } from 'node:url'

import { EditorLayoutInfoComputer } from '../../node_modules/monaco-editor/esm/vs/editor/common/config/editorOptions.js'
import { MinimapCharRenderer } from '../../node_modules/monaco-editor/esm/vs/editor/browser/viewParts/minimap/minimapCharRenderer.js'
import { prebakedMiniMaps } from '../../node_modules/monaco-editor/esm/vs/editor/browser/viewParts/minimap/minimapPreBaked.js'

const ROOT = new URL('../../', import.meta.url)
const SOURCE = new URL('node_modules/monaco-editor/esm/vs/editor/browser/viewParts/minimap/minimap.js', ROOT)
const FIXTURE = new URL('native/taide-native-ui/tests/fixtures/minimap-reference.txt', ROOT)
const ASSETS = new URL('native/taide-native-ui/resources/minimap/', ROOT)
const EXTRACTED_LAYOUT = '/private/tmp/taide-original-minimap-layout-20261009.mjs'
const WIDTHS = [80, 400, 1200]
const HEIGHTS = [80, 240, 800]
const RATIOS = [1, 1.25, 2, 3]
const LINE_COUNTS = [1, 20, 200, 10000]
const SCROLL_FRACTIONS = [0, 0.001, 0.25, 0.5, 0.99, 1, 0.75, 0.4, 0]
const LINE_HEIGHT = 20
const CHARACTER_WIDTH = 8
const GUTTER_WIDTH = 64
const SCROLLBAR_WIDTH = 14
const ALPHAS = [255, 127]
const GLYPH_CODES = [...Array.from({ length: 95 }, (_, index) => index + 32), 9, 129, 0x4e00, 0xd83d, 0xde00, 0xfffd]
const COLORS = [
    { foreground: { r: 212, g: 212, b: 212 }, background: { r: 30, g: 30, b: 30 }, light: false },
    { foreground: { r: 0, g: 128, b: 0 }, background: { r: 255, g: 255, b: 255 }, light: true },
]

const original = readFileSync(SOURCE, 'utf8')
const layoutStart = original.indexOf('class MinimapLayout {')
const layoutEnd = original.indexOf('\nclass MinimapLine {', layoutStart)
if (layoutStart < 0 || layoutEnd < layoutStart) throw new Error('Original minimap layout boundary changed')
const layoutModule = original.slice(layoutStart, layoutEnd) + '\nexport { MinimapLayout };\n'
writeFileSync(EXTRACTED_LAYOUT, layoutModule)
const { MinimapLayout } = await import(pathToFileURL(EXTRACTED_LAYOUT).href)
const dimensions = []
const layouts = []
for (const width of WIDTHS) {
    for (const height of HEIGHTS) {
        for (const pixelRatio of RATIOS) {
            const input = {
                outerWidth: width,
                outerHeight: height,
                pixelRatio,
                lineHeight: LINE_HEIGHT,
                typicalHalfwidthCharacterWidth: CHARACTER_WIDTH,
                scrollBeyondLastLine: true,
                paddingTop: 0,
                paddingBottom: 0,
                minimap: {
                    enabled: true,
                    side: 'right',
                    size: 'proportional',
                    showSlider: 'mouseover',
                    renderCharacters: true,
                    maxColumn: 120,
                    scale: 1,
                },
                verticalScrollbarWidth: SCROLLBAR_WIDTH,
                viewLineCount: 200,
                remainingWidth: Math.max(0, width - GUTTER_WIDTH),
                isViewportWrapping: false,
            }
            const computed = EditorLayoutInfoComputer._computeMinimapLayout(input, {})
            const dimensionIndex = dimensions.length
            dimensions.push({ input, output: computed })
            const options = {
                ...computed,
                pixelRatio,
                lineHeight: LINE_HEIGHT,
                paddingTop: 0,
                paddingBottom: 0,
                scrollBeyondLastLine: true,
                minimapHeight: height,
                canvasInnerHeight: computed.minimapCanvasInnerHeight,
            }
            for (const count of LINE_COUNTS) {
                for (const beyond of [false, true]) {
                    const scrollHeight = Math.max(height, count * LINE_HEIGHT + (beyond ? height - LINE_HEIGHT : 0))
                    let previous
                    let previousIndex = null
                    for (const fraction of SCROLL_FRACTIONS) {
                        const scrollTop = (scrollHeight - height) * fraction
                        const first = Math.min(count, Math.floor(scrollTop / LINE_HEIGHT) + 1)
                        const last = Math.min(count, Math.ceil((scrollTop + height) / LINE_HEIGHT))
                        const output = MinimapLayout.create(
                            { ...options, scrollBeyondLastLine: beyond },
                            first,
                            last,
                            (first - 1) * LINE_HEIGHT,
                            height,
                            false,
                            count,
                            count,
                            scrollTop,
                            scrollHeight,
                            previous,
                        )
                        layouts.push({ dimensionIndex, count, beyond, scrollTop, scrollHeight, first, last, previousIndex, output })
                        previous = output
                        previousIndex = layouts.length - 1
                    }
                }
            }
        }
    }
}

mkdirSync(ASSETS, { recursive: true })
const glyphs = []
for (const scale of [1, 2]) {
    const data = prebakedMiniMaps[scale]()
    writeFileSync(new URL(`scale-${scale}.bin`, ASSETS), data)
    const renderer = new MinimapCharRenderer(data, scale)
    for (const code of GLYPH_CODES) {
        for (const { foreground, background, light } of COLORS) {
            for (const alpha of ALPHAS) {
                const target = { width: scale, height: scale * 2, data: new Uint8ClampedArray(scale * scale * 2 * 4) }
                renderer.renderChar(target, 0, 0, code, foreground, alpha, background, 255, scale, light, false)
                glyphs.push({ scale, code, foreground, background, light, alpha, rgba: Array.from(target.data) })
            }
        }
    }
}
mkdirSync(new URL('.', FIXTURE), { recursive: true })
writeFileSync(FIXTURE, JSON.stringify({ monaco: '0.56.0', dimensions, layouts, glyphs }) + '\n')
console.info(JSON.stringify({ fixture: fileURLToPath(FIXTURE), dimensions: dimensions.length, layouts: layouts.length, glyphs: glyphs.length }))
process.exit(0)
