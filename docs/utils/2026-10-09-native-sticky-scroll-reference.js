import { writeFileSync } from 'node:fs'
import { GlobalRegistrator } from '@happy-dom/global-registrator'

GlobalRegistrator.register({ settings: { disableCSSFileLoading: true } })
const { StickyLineCandidateProvider } =
    await import('../../node_modules/monaco-editor/esm/vs/editor/contrib/stickyScroll/browser/stickyScrollProvider.js')
const { StickyScrollController } =
    await import('../../node_modules/monaco-editor/esm/vs/editor/contrib/stickyScroll/browser/stickyScrollController.js')
const { StickyElement, StickyRange } =
    await import('../../node_modules/monaco-editor/esm/vs/editor/contrib/stickyScroll/browser/stickyScrollElement.js')
const LINE_HEIGHT = 20
const LINE_COUNT = 40
const MAX_STICKY_LINES = 5
const HEIGHT_FRACTION = 0.25
const HEIGHTS = [80, 160, 480]
const SCROLL_POSITIONS = [0, 1, 19, 20, 39, 40, 61, 100, 159, 200, 279, 300, 399, 499, 599, 639, 719]
const MODELS = [
    { scopes: [], hidden: [], wraps: [] },
    { scopes: [[0, 30]], hidden: [], wraps: [] },
    {
        scopes: [
            [0, 30],
            [2, 25],
            [4, 22],
            [6, 19],
            [8, 16],
            [10, 14],
        ],
        hidden: [],
        wraps: [],
    },
    {
        scopes: [
            [0, 12],
            [2, 8],
            [15, 35],
            [19, 28],
        ],
        hidden: [],
        wraps: [],
    },
    {
        scopes: [
            [0, 30],
            [2, 20],
            [5, 14],
        ],
        hidden: [[3, 15]],
        wraps: [],
    },
    {
        scopes: [
            [0, 30],
            [3, 25],
            [7, 17],
        ],
        hidden: [],
        wraps: [
            [0, 3],
            [3, 2],
            [9, 4],
        ],
    },
    {
        scopes: [
            [0, 30],
            [3, 25],
            [7, 17],
        ],
        hidden: [[8, 18]],
        wraps: [
            [0, 3],
            [3, 2],
        ],
    },
]

const reference = (definition, height, scrollTop) => {
    const nodes = definition.scopes.map(([start, end]) => new StickyElement(new StickyRange(start + 1, end + 1), [], undefined))
    const root = new StickyElement(undefined, [], undefined)
    for (const [index, node] of nodes.entries()) {
        const parent =
            nodes
                .slice(0, index)
                .filter(
                    (candidate) =>
                        candidate.range.startLineNumber <= node.range.startLineNumber && candidate.range.endLineNumber >= node.range.endLineNumber,
                )
                .at(-1) ?? root
        parent.children = [...parent.children, node]
        node.parent = parent
    }
    const rows = Array.from({ length: LINE_COUNT }, (_, line) => {
        if (definition.hidden.some(([start, end]) => start <= line && line < end)) return []
        const count = definition.wraps.find(([wrapped]) => wrapped === line)?.[1] ?? 1
        return Array.from({ length: count }, () => line)
    }).flat()
    const tops = Array.from({ length: LINE_COUNT }, (_, line) => {
        const index = rows.indexOf(line)
        if (index >= 0) return index * LINE_HEIGHT
        const previous = rows.findLastIndex((candidate) => candidate < line)
        return Math.max(previous, 0) * LINE_HEIGHT
    })
    const bottoms = Array.from({ length: LINE_COUNT }, (_, line) => {
        const index = rows.lastIndexOf(line)
        return index < 0 ? tops[line] + LINE_HEIGHT : (index + 1) * LINE_HEIGHT
    })
    const visible = rows.slice(Math.floor(scrollTop / LINE_HEIGHT), Math.ceil((scrollTop + height) / LINE_HEIGHT))
    const visibleLines = visible.length === 0 ? [0, 0] : [visible[0], visible.at(-1) + 1]
    const textModel = { isValidRange: (range) => range.startLineNumber >= 1 && range.endLineNumber <= LINE_COUNT }
    const provider = Object.create(StickyLineCandidateProvider.prototype)
    provider._model = { element: root }
    provider._editor = {
        getModel: () => textModel,
        getLineHeightForPosition: () => LINE_HEIGHT,
        _getViewModel: () => ({
            getHiddenAreas: () => definition.hidden.map(([start, end]) => ({ startLineNumber: start + 1, endLineNumber: end })),
        }),
    }
    const context = {
        _maxStickyLines: Math.round((height / LINE_HEIGHT) * HEIGHT_FRACTION),
        _stickyLineCandidateProvider: provider,
        _showEndForLine: null,
        _editor: {
            getOption: () => ({ maxLineCount: MAX_STICKY_LINES }),
            getScrollTop: () => scrollTop,
            getVisibleRanges: () => (visible.length === 0 ? [] : [{ startLineNumber: visibleLines[0] + 1, endLineNumber: visibleLines[1] }]),
            getTopForLineNumber: (line) => tops[line - 1],
            getBottomForLineNumber: (line) => bottoms[line - 1],
        },
    }
    const candidates =
        visible.length === 0
            ? []
            : provider
                  .getCandidateStickyLinesIntersecting(new StickyRange(visibleLines[0] + 1, visibleLines[1]))
                  .map((candidate) => [candidate.startLineNumber - 1, candidate.endLineNumber, candidate.top / LINE_HEIGHT])
    const state = StickyScrollController.prototype.findScrollWidgetState.call(context)
    return {
        ...definition,
        lineCount: LINE_COUNT,
        height,
        scrollTop,
        visibleLines,
        tops,
        bottoms,
        candidates,
        starts: state.startLineNumbers.map((line) => line - 1),
        ends: state.endLineNumbers.map((line) => line - 1),
        lastRelativePosition: state.lastLineRelativePosition,
    }
}

const cases = MODELS.flatMap((model) => HEIGHTS.flatMap((height) => SCROLL_POSITIONS.map((scrollTop) => reference(model, height, scrollTop))))
const output = new URL('../../native/taide-native-editor/tests/fixtures/sticky-scroll-reference.txt', import.meta.url)
const records = cases.map((sample) =>
    [
        sample.lineCount,
        sample.height,
        sample.scrollTop,
        ...sample.visibleLines,
        sample.scopes.flat().join(','),
        sample.hidden.flat().join(','),
        sample.tops.join(','),
        sample.bottoms.join(','),
        sample.candidates.flat().join(','),
        sample.starts.join(','),
        sample.ends.join(','),
        sample.lastRelativePosition,
    ].join('\t'),
)
writeFileSync(output, `0.56.0\t${LINE_HEIGHT}\t${cases.length}\n${records.join('\n')}\n`)
console.info(JSON.stringify({ cases: cases.length, source: 'StickyLineCandidateProvider/StickyScrollController' }))
await GlobalRegistrator.unregister()
process.exit(0)
