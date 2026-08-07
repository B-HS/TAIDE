import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import type { PDFDocumentProxy, RenderTask } from 'pdfjs-dist'
import { ChevronLeft, ChevronRight, FileWarning, Loader2, ZoomIn, ZoomOut } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { pdfjs } from '@shared/lib/pdf/setup'
import { Button } from '@shared/ui/button'
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

    const [status, setStatus] = useState<PdfPreviewStatus>('loading')
    const [numPages, setNumPages] = useState(0)
    const [currentPage, setCurrentPage] = useState(1)
    const [scale, setScale] = useState(PDF_INITIAL_SCALE)

    const { t } = useTranslation()

    const goToPreviousPage = () => setCurrentPage((page) => Math.max(1, page - 1))
    const goToNextPage = () => setCurrentPage((page) => Math.min(numPages, page + 1))
    const zoomIn = () => setScale((value) => Math.min(PDF_MAX_SCALE, value + PDF_SCALE_STEP))
    const zoomOut = () => setScale((value) => Math.max(PDF_MIN_SCALE, value - PDF_SCALE_STEP))

    useEffect(() => {
        let cancelled = false
        const loadingTask = pdfjs.getDocument({ data: data.slice(0) })

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

        const renderPage = async () => {
            const page = await pdfDocument.getPage(currentPage)
            if (cancelled) return
            const viewport = page.getViewport({ scale })
            canvas.width = viewport.width
            canvas.height = viewport.height

            const renderTask = page.render({ canvas, viewport })
            renderTaskRef.current = renderTask
            renderTask.promise.catch(() => undefined)
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
                <Button
                    type='button'
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('preview.pdf.previousPage')}
                    disabled={currentPage <= 1}
                    onClick={goToPreviousPage}>
                    <ChevronLeft className='size-3.5' />
                </Button>
                <span>{t('preview.pdf.pageIndicator', { current: currentPage, total: numPages })}</span>
                <Button
                    type='button'
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('preview.pdf.nextPage')}
                    disabled={currentPage >= numPages}
                    onClick={goToNextPage}>
                    <ChevronRight className='size-3.5' />
                </Button>
                <span className='bg-editor-widget-border mx-1 h-4 w-px' />
                <Button
                    type='button'
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('preview.pdf.zoomOut')}
                    disabled={scale <= PDF_MIN_SCALE}
                    onClick={zoomOut}>
                    <ZoomOut className='size-3.5' />
                </Button>
                <span>{Math.round(scale * 100)}%</span>
                <Button
                    type='button'
                    variant='ghost'
                    size='icon-xs'
                    aria-label={t('preview.pdf.zoomIn')}
                    disabled={scale >= PDF_MAX_SCALE}
                    onClick={zoomIn}>
                    <ZoomIn className='size-3.5' />
                </Button>
            </div>
            <ScrollContainer className='flex-1' orientation='both'>
                <canvas ref={canvasRef} className='mx-auto my-4 block shadow' />
            </ScrollContainer>
        </div>
    )
}
