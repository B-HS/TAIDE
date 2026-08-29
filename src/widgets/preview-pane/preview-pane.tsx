import type { FC, ReactNode } from 'react'
import { lazy, Suspense } from 'react'
import { useQuery } from '@tanstack/react-query'
import { convertFileSrc } from '@tauri-apps/api/core'
import { toast } from 'sonner'
import { describeIpcError } from '@shared/lib/ipc-error-message'
import { resolvePreviewKind, resolvePreviewMimeType } from '@shared/lib/preview-kind'
import { fileNameOf } from '@shared/lib/relative-path'
import { useObjectUrl } from '@shared/hooks/use-object-url'
import { fileRawQueryOptions } from '@entities/file/file.query'
import { systemOpenPath } from '@entities/system/system.ipc'
import { ImagePreview } from '@features/preview/image-preview'
import { VideoPreview } from '@features/preview/video-preview'
import { AudioPreview } from '@features/preview/audio-preview'
import { HtmlPreview } from '@features/preview/html-preview'
import { UnsupportedPreview } from '@features/preview/unsupported-preview'

const DEFAULT_BLOB_MIME_TYPE = 'application/octet-stream'

/**
 * The four document previews are the app's heaviest leaf dependencies — `pdfjs-dist`, `xlsx` and
 * `@rhwp/core` together dominated the single eager entry chunk (audit §1-1), even though a session
 * that never opens a `.pdf`/`.xlsx`/`.hwp`/`.pptx` file never renders any of them. Splitting them
 * here (rather than at `pane-node-view.tsx`, which cannot see the preview kind) keeps the whole
 * decode/render stack of each format out of the boot payload and off the main thread until the
 * matching file is actually opened. Image/video/audio/HTML previews stay eager: they are thin
 * wrappers over native elements with no library behind them.
 */
const PdfPreview = lazy(async () => ({ default: (await import('@features/preview/pdf-preview')).PdfPreview }))
const SpreadsheetPreview = lazy(async () => ({ default: (await import('@features/preview/spreadsheet-preview')).SpreadsheetPreview }))
const HwpPreview = lazy(async () => ({ default: (await import('@features/preview/hwp-preview')).HwpPreview }))
const PresentationPreview = lazy(async () => ({ default: (await import('@features/preview/presentation-preview')).PresentationPreview }))

/**
 * The fallback matches the pending-bytes placeholder below, so the chunk fetch reads as the same
 * blank editor surface the raw-bytes query already shows rather than as a second, different
 * loading state.
 */
const withPreviewChunkSuspense = (preview: ReactNode) => (
    <Suspense fallback={<div className='bg-editor-background h-full w-full' />}>{preview}</Suspense>
)

export type PreviewPaneProps = {
    path: string
}

export const PreviewPane: FC<PreviewPaneProps> = ({ path }) => {
    const fileName = fileNameOf(path)
    const kind = resolvePreviewKind(fileName)
    const needsObjectUrl = kind === 'image' || kind === 'html'
    const needsBuffer = kind === 'pdf' || kind === 'spreadsheet' || kind === 'hwp' || kind === 'presentation'
    const needsRawBytes = needsObjectUrl || needsBuffer
    const mimeType = resolvePreviewMimeType(fileName) ?? DEFAULT_BLOB_MIME_TYPE

    const { data, isPending, isError } = useQuery({ ...fileRawQueryOptions(path), enabled: needsRawBytes })
    const objectUrl = useObjectUrl(needsObjectUrl ? data : undefined, mimeType)

    const handleOpenExternal = () => void systemOpenPath(path).catch((error: Error) => toast.error(describeIpcError(error)))

    if (kind === 'video') return <VideoPreview src={convertFileSrc(path)} />
    if (kind === 'audio') return <AudioPreview src={convertFileSrc(path)} fileName={fileName} />

    if (needsRawBytes && isError) return <UnsupportedPreview fileName={fileName} onOpenExternal={handleOpenExternal} />

    if (needsObjectUrl) {
        if (isPending || !objectUrl) return <div className='bg-editor-background h-full w-full' />
        if (kind === 'image') return <ImagePreview src={objectUrl} alt={fileName} />
        return <HtmlPreview src={objectUrl} title={fileName} />
    }

    if (needsBuffer) {
        if (isPending || !data) return <div className='bg-editor-background h-full w-full' />
        if (kind === 'pdf') return withPreviewChunkSuspense(<PdfPreview data={data} onOpenExternally={handleOpenExternal} />)
        if (kind === 'spreadsheet') return withPreviewChunkSuspense(<SpreadsheetPreview data={data} onOpenExternally={handleOpenExternal} />)
        if (kind === 'presentation') return withPreviewChunkSuspense(<PresentationPreview data={data} onOpenExternally={handleOpenExternal} />)
        return withPreviewChunkSuspense(<HwpPreview data={data} onOpenExternally={handleOpenExternal} />)
    }

    return <UnsupportedPreview fileName={fileName} onOpenExternal={handleOpenExternal} />
}
