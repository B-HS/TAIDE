import type { FC } from 'react'
import { useEffect, useState } from 'react'
import { AlertTriangle, FileWarning, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@shared/lib/cn'
import { parsePptxOutline, type PptxOutline } from '@shared/lib/pptx-outline'
import { PreviewStatusMessage } from '@features/preview/preview-status'

export type PresentationPreviewProps = {
    data: ArrayBuffer
    onOpenExternally: () => void
}

type PresentationPreviewStatus = 'loading' | 'ready' | 'error'

export const PresentationPreview: FC<PresentationPreviewProps> = ({ data, onOpenExternally }) => {
    const { t } = useTranslation()
    const [status, setStatus] = useState<PresentationPreviewStatus>('loading')
    const [outline, setOutline] = useState<PptxOutline | null>(null)
    const [selectedIndex, setSelectedIndex] = useState(0)

    useEffect(() => {
        let cancelled = false

        parsePptxOutline(data)
            .then((parsed) => {
                if (cancelled) return
                setOutline(parsed)
                setSelectedIndex(0)
                setStatus('ready')
            })
            .catch(() => {
                if (!cancelled) setStatus('error')
            })

        return () => {
            cancelled = true
        }
    }, [data])

    if (status === 'loading') {
        return <PreviewStatusMessage icon={<Loader2 className='size-5 animate-spin' />} message={t('common.loading')} />
    }

    if (status === 'error' || !outline) {
        return (
            <PreviewStatusMessage
                icon={<FileWarning className='size-5' />}
                message={t('preview.presentation.loadFailed')}
                actionLabel={t('preview.openExternally')}
                onAction={onOpenExternally}
            />
        )
    }

    const clampedIndex = Math.min(selectedIndex, outline.slides.length - 1)
    const activeSlide = outline.slides[clampedIndex] ?? null

    return (
        <div className='bg-editor-background text-editor-foreground flex h-full w-full flex-col'>
            <div className='bg-status-warning/15 text-status-warning flex shrink-0 items-center gap-2 px-3 py-1.5 text-xs'>
                <AlertTriangle className='size-3.5 shrink-0' />
                <span>{t('preview.presentation.layoutDisclaimer')}</span>
            </div>
            <div className='flex min-h-0 flex-1'>
                <div className='border-app-border flex w-48 shrink-0 flex-col overflow-y-auto border-r py-1'>
                    {outline.slides.map((slide, position) => (
                        <button
                            key={slide.index}
                            type='button'
                            onClick={() => setSelectedIndex(position)}
                            className={cn(
                                'hover:bg-explorer-item-hover truncate px-3 py-1.5 text-left text-xs',
                                position === clampedIndex && 'bg-explorer-item-selected',
                            )}>
                            {t('preview.presentation.slideLabel', { index: slide.index })}
                        </button>
                    ))}
                </div>
                <div className='min-h-0 flex-1 overflow-y-auto px-4 py-3'>
                    {activeSlide && activeSlide.paragraphs.length > 0 && (
                        <ul className='flex flex-col gap-2 text-sm'>
                            {activeSlide.paragraphs.map((paragraph, position) => (
                                <li key={position}>{paragraph}</li>
                            ))}
                        </ul>
                    )}
                    {activeSlide && activeSlide.paragraphs.length === 0 && <p className='text-xs opacity-60'>{t('preview.presentation.noText')}</p>}
                </div>
            </div>
        </div>
    )
}
