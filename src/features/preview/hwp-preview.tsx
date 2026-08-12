import type { FC } from 'react'
import { useEffect, useRef, useState } from 'react'
import init, { HwpDocument } from '@rhwp/core'
import wasmUrl from '@rhwp/core/rhwp_bg.wasm?url'
import { ChevronLeft, ChevronRight, FileWarning, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@shared/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'
import { ScrollContainer } from '@shared/scroll/scroll-container'
import { PreviewStatusMessage } from '@features/preview/preview-status'

export type HwpPreviewProps = {
    data: ArrayBuffer
    onOpenExternally: () => void
}

type HwpPreviewStatus = 'loading' | 'ready' | 'error'

declare global {
    var measureTextWidth: (font: string, text: string) => number
}

let measureContext: CanvasRenderingContext2D | null = null
let measureContextFont = ''

globalThis.measureTextWidth = (font, text) => {
    measureContext ??= document.createElement('canvas').getContext('2d')
    if (!measureContext) return 0
    if (font !== measureContextFont) {
        measureContext.font = font
        measureContextFont = font
    }
    return measureContext.measureText(text).width
}

let wasmReadyPromise: Promise<void> | null = null

const initWasm = async () => {
    await init({ module_or_path: wasmUrl })
}

const ensureWasmReady = () => {
    wasmReadyPromise ??= initWasm()
    return wasmReadyPromise
}

export const HwpPreview: FC<HwpPreviewProps> = ({ data, onOpenExternally }) => {
    const documentRef = useRef<HwpDocument | null>(null)

    const [status, setStatus] = useState<HwpPreviewStatus>('loading')
    const [pageCount, setPageCount] = useState(0)
    const [currentPage, setCurrentPage] = useState(0)
    const [pageImageUrl, setPageImageUrl] = useState<string | null>(null)

    const { t } = useTranslation()

    const goToPreviousPage = () => setCurrentPage((page) => Math.max(0, page - 1))
    const goToNextPage = () => setCurrentPage((page) => Math.min(pageCount - 1, page + 1))

    useEffect(() => {
        let cancelled = false

        const load = async () => {
            try {
                await ensureWasmReady()
                if (cancelled) return

                const hwpDocument = new HwpDocument(new Uint8Array(data))
                if (cancelled) {
                    hwpDocument.free()
                    return
                }

                documentRef.current = hwpDocument
                setPageCount(hwpDocument.pageCount())
                setCurrentPage(0)
                setStatus('ready')
            } catch {
                if (!cancelled) setStatus('error')
            }
        }
        void load()

        return () => {
            cancelled = true
            documentRef.current?.free()
            documentRef.current = null
        }
    }, [data])

    useEffect(() => {
        const hwpDocument = documentRef.current
        if (status !== 'ready' || !hwpDocument || pageCount === 0) return

        let objectUrl: string | null = null
        try {
            const svg = hwpDocument.renderPageSvg(currentPage)
            objectUrl = URL.createObjectURL(new Blob([svg], { type: 'image/svg+xml' }))
            setPageImageUrl(objectUrl)
        } catch {
            setPageImageUrl(null)
        }

        return () => {
            if (objectUrl) URL.revokeObjectURL(objectUrl)
        }
    }, [status, currentPage, pageCount])

    if (status === 'loading') {
        return <PreviewStatusMessage icon={<Loader2 className='size-5 animate-spin' />} message={t('common.loading')} />
    }

    if (status === 'error') {
        return (
            <PreviewStatusMessage
                icon={<FileWarning className='size-5' />}
                message={t('preview.hwp.loadFailed')}
                actionLabel={t('preview.openExternally')}
                onAction={onOpenExternally}
            />
        )
    }

    if (pageCount === 0) {
        return <PreviewStatusMessage icon={<FileWarning className='size-5' />} message={t('preview.hwp.noPages')} />
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
                            aria-label={t('preview.hwp.previousPage')}
                            disabled={currentPage <= 0}
                            onClick={goToPreviousPage}>
                            <ChevronLeft className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.hwp.previousPage')}</TooltipContent>
                </Tooltip>
                <span>{t('preview.hwp.pageIndicator', { current: currentPage + 1, total: pageCount })}</span>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <Button
                            type='button'
                            variant='ghost'
                            size='icon-xs'
                            aria-label={t('preview.hwp.nextPage')}
                            disabled={currentPage >= pageCount - 1}
                            onClick={goToNextPage}>
                            <ChevronRight className='size-3.5' />
                        </Button>
                    </TooltipTrigger>
                    <TooltipContent side='bottom'>{t('preview.hwp.nextPage')}</TooltipContent>
                </Tooltip>
            </div>
            <ScrollContainer className='flex-1' orientation='both'>
                {pageImageUrl && (
                    <img
                        src={pageImageUrl}
                        alt={t('preview.hwp.pageIndicator', { current: currentPage + 1, total: pageCount })}
                        className='mx-auto my-4 block max-w-full'
                    />
                )}
            </ScrollContainer>
        </div>
    )
}
