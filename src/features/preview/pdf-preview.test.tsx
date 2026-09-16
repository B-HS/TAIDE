import { describe, expect, mock, test } from 'bun:test'
import { act, renderWithProviders, screen } from '@shared/testing/render'
import { TooltipProvider } from '@shared/ui/tooltip'

/**
 * `@shared/lib/pdf/setup` spawns pdf.js's real `?worker` bundle, which only Vite can resolve — and
 * `PdfPreview` is its only consumer, so stubbing it here costs no other suite anything
 * (`mock.module` is process-wide, `docs/memory/test-conventions.md` §3). The fake keeps the shapes
 * the component actually drives: a loading task that can be destroyed, a document whose `getPage`
 * this test controls per instance, and a page that hands back a viewport and a render task.
 *
 * `TooltipProvider` is supplied by the test rather than `renderWithProviders` — the ready-state
 * toolbar is built from `shared/ui/tooltip`, whose `Tooltip` renders a bare radix `Root` and throws
 * without a provider above it.
 */
type FakeDocument = { numPages: number; getPage: (page: number) => Promise<unknown>; getPageCalls: number[] }

const pdfState: { documents: FakeDocument[]; nextIndex: number } = { documents: [], nextIndex: 0 }

const createFakePage = () => ({
    getViewport: ({ scale }: { scale: number }) => ({ width: 100 * scale, height: 200 * scale }),
    render: () => ({ promise: Promise.resolve(), cancel: () => {} }),
})

const createFakeDocument = (getPage?: (page: number) => Promise<unknown>): FakeDocument => {
    const document: FakeDocument = {
        numPages: 3,
        getPageCalls: [],
        getPage: async (page: number) => {
            document.getPageCalls.push(page)
            return getPage ? await getPage(page) : createFakePage()
        },
    }
    return document
}

mock.module('@shared/lib/pdf/setup', () => ({
    getPdfjsWithWorker: () => ({
        getDocument: () => {
            const document = pdfState.documents[pdfState.nextIndex] ?? createFakeDocument()
            pdfState.nextIndex += 1
            return { promise: Promise.resolve(document), destroy: async () => {} }
        },
    }),
}))

const importPdfPreview = () => import('@features/preview/pdf-preview')

const MICROTASK_FLUSH_ROUNDS = 8

/** happy-dom delivers React's effect/promise work on the microtask queue, so draining it is both enough and more deterministic than a DOM `waitFor` (`docs/memory/test-conventions.md` §5). */
const flushMicrotasks = async () => {
    await act(async () => {
        for (let round = 0; round < MICROTASK_FLUSH_ROUNDS; round += 1) await Promise.resolve()
    })
}

const renderPdfPreview = async (documents: FakeDocument[]) => {
    const { PdfPreview } = await importPdfPreview()
    pdfState.documents = documents
    pdfState.nextIndex = 0

    const renderWith = (data: ArrayBuffer) => (
        <TooltipProvider>
            <PdfPreview data={data} onOpenExternally={() => {}} />
        </TooltipProvider>
    )
    const result = renderWithProviders(renderWith(new ArrayBuffer(8)))
    await flushMicrotasks()

    return { swapData: () => result.rerender(renderWith(new ArrayBuffer(16))) }
}

describe('PdfPreview 문서 교체', () => {
    test('외부 편집으로 data 가 바뀌면 새 문서의 페이지를 다시 렌더한다', async () => {
        const first = createFakeDocument()
        const second = createFakeDocument()
        const { swapData } = await renderPdfPreview([first, second])
        expect(first.getPageCalls).toEqual([1])

        act(() => swapData())
        expect(screen.getByText('common.loading')).toBeDefined()

        await flushMicrotasks()
        expect(second.getPageCalls).toEqual([1])
    })
})

describe('PdfPreview 렌더 실패', () => {
    test('getPage 가 거부되면 삼키지 않고 오류 상태로 넘어간다', async () => {
        const document = createFakeDocument(() => Promise.reject(new Error('Transport destroyed')))
        await renderPdfPreview([document])

        expect(document.getPageCalls).toEqual([1])
        expect(screen.getByText('preview.pdf.loadFailed')).toBeDefined()
    })
})
