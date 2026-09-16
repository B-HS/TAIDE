import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { PDFDocumentProxy, RenderTask } from 'pdfjs-dist'
import { ChevronLeft, ChevronRight, FileWarning, Loader2, ZoomIn, ZoomOut } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { getPdfjsWithWorker } from '@shared/lib/pdf/setup'
import { Button } from '@shared/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { PreviewStatusMessage } from '@features/preview/preview-status'

export type PdfPreviewProps = {
    data: ArrayBuffer
    onOpenExternally: () => void
}

type PdfPreviewStatus = 'loading' | 'ready' | 'error'

const PDF_MIN_SCALE = 0.5
const PDF_MAX_SCALE = 3
const PDF_SCALE_STEP = 0.25
const PDF_INITIAL_SCALE = 1

export const PdfPreview: FC<PdfPreviewProps> = ({ data, onOpenExternally }) => {
    const canvasRef = useRef<HTMLCanvasElement>(null)
    const documentRef = useRef<PDFDocumentProxy | null>(null)
    const renderTaskRef = useRef<RenderTask | null>(null)

    const [loadedData, setLoadedData] = useState(data)
    const [status, setStatus] = useState<PdfPreviewStatus>('loading')
    const [numPages, setNumPages] = useState(0)
    const [currentPage, setCurrentPage] = useState(1)
    const [scale, setScale] = useState(PDF_INITIAL_SCALE)

    const { t } = useTranslation()

    const goToPreviousPage = () => setCurrentPage((page) => Math.max(1, page - 1))
    const goToNextPage = () => setCurrentPage((page) => Math.min(numPages, page + 1))
    const zoomIn = () => setScale((value) => Math.min(PDF_MAX_SCALE, value + PDF_SCALE_STEP))
    const zoomOut = () => setScale((value) => Math.max(PDF_MIN_SCALE, value - PDF_SCALE_STEP))

    /**
     * Render-phase adjustment (React's documented "adjust state when a prop changes", the same shape
     * `editor-pane.tsx` uses) rather than a `setStatus` inside the load effect below, which the
     * React Compiler lint rejects as a cascading render.
     *
     * Re-entering `loading` is what makes a swapped `data` actually repaint. The render effect keys
     * off `[status, currentPage, scale]`, none of which `data` touches, and a reload that lands back
     * on page 1 at 100% sets all three to the values they already held — so without this reset the
     * effect never re-ran and the canvas kept the *previous* document's pixels (an external save to
     * the previewed file refetches `fileRawQueryOptions` into a new `ArrayBuffer`). Dropping back to
     * `loading` also unmounts the canvas for that frame, which closes the window where `status` said
     * `ready` while the load effect's cleanup had already nulled `documentRef.current`.
     */
    if (loadedData !== data) {
        setLoadedData(data)
        setStatus('loading')
    }

    useEffect(() => {
        let cancelled = false
        const loadingTask = getPdfjsWithWorker().getDocument({ data: data.slice(0) })

        const load = async () => {
            try {
                const pdfDocument = await loadingTask.promise
                if (cancelled) return
                documentRef.current = pdfDocument
                setNumPages(pdfDocument.numPages)
                setCurrentPage(1)
                setScale(PDF_INITIAL_SCALE)
                setStatus('ready')
            } catch {
                if (!cancelled) setStatus('error')
            }
        }
        void load()

        return () => {
            cancelled = true
            documentRef.current = null
            void loadingTask.destroy()
        }
    }, [data])

    useEffect(() => {
        const pdfDocument = documentRef.current
        const canvas = canvasRef.current
        if (status !== 'ready' || !pdfDocument || !canvas) return

        let cancelled = false

        /**
         * `getPage` rejects with "Transport destroyed" whenever the load effect's cleanup destroys the
         * loading task while a page request is still in flight — closing or switching away from the tab
         * right after a page/zoom change, or the same external-save refetch that swaps `data`. The
         * `cancelled` flag is only read *after* the await resolves, so it cannot cover that path; this
         * `try` is what keeps it from becoming an unhandled rejection, and mirrors the load effect's own
         * catch by surfacing a genuine (non-cancelled) failure as `error` rather than a frozen canvas.
         */
        const renderPage = async () => {
            try {
                const page = await pdfDocument.getPage(currentPage)
                if (cancelled) return
                const viewport = page.getViewport({ scale })
                canvas.width = viewport.width
                canvas.height = viewport.height

                const renderTask = page.render({ canvas, viewport })
                renderTaskRef.current = renderTask
                renderTask.promise.catch(() => undefined)
            } catch {
                if (!cancelled) setStatus('error')
            }
        }
        void renderPage()

        return () => {
            cancelled = true
            renderTaskRef.current?.cancel()
        }
    }, [status, currentPage, scale])

    if (status === 'loading') {
        return <PreviewStatusMessage icon={<Loader2 className='size-5 animate-spin' />} message={t('common.loading')} />
    }

    if (status === 'error') {
        return (
            <PreviewStatusMessage
                icon={<FileWarning className='size-5' />}
                message={t('preview.pdf.loadFailed')}
                actionLabel={t('preview.openExternally')}
                onAction={onOpenExternally}
            />
        )
    }

    return (
        <div className='bg-editor-background flex h-full w-full flex-col'>
            <div className='border-editor-widget-border bg-editor-widget-background text-editor-foreground flex shrink-0 items-center justify-center gap-2 border-b px-3 py-1.5 text-xs'>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            type='button'
                            variant='ghost'
                            size='icon-xs'
                            aria-label={t('preview.pdf.previousPage')}
                            disabled={currentPage <= 1}
                            onClick={goToPreviousPage}>
                            <ChevronLeft className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.pdf.previousPage')}</TooltipContent>
                </Tooltip>
                <span>{t('preview.pdf.pageIndicator', { current: currentPage, total: numPages })}</span>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            type='button'
                            variant='ghost'
                            size='icon-xs'
                            aria-label={t('preview.pdf.nextPage')}
                            disabled={currentPage >= numPages}
                            onClick={goToNextPage}>
                            <ChevronRight className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.pdf.nextPage')}</TooltipContent>
                </Tooltip>
                <span className='bg-editor-widget-border mx-1 h-4 w-px' />
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            type='button'
                            variant='ghost'
                            size='icon-xs'
                            aria-label={t('preview.pdf.zoomOut')}
                            disabled={scale <= PDF_MIN_SCALE}
                            onClick={zoomOut}>
                            <ZoomOut className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.pdf.zoomOut')}</TooltipContent>
                </Tooltip>
                <span>{Math.round(scale * 100)}%</span>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            type='button'
                            variant='ghost'
                            size='icon-xs'
                            aria-label={t('preview.pdf.zoomIn')}
                            disabled={scale >= PDF_MAX_SCALE}
                            onClick={zoomIn}>
                            <ZoomIn className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.pdf.zoomIn')}</TooltipContent>
                </Tooltip>
            </div>
            <ScrollContainer className='flex-1' orientation='both'>
                <canvas ref={canvasRef} className='mx-auto my-4 block shadow' />
            </ScrollContainer>
        </div>
    )
}
