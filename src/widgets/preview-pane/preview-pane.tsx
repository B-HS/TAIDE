import type { FC } from 'react'
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
import { PdfPreview } from '@features/preview/pdf-preview'
import { SpreadsheetPreview } from '@features/preview/spreadsheet-preview'
import { HwpPreview } from '@features/preview/hwp-preview'
import { PresentationPreview } from '@features/preview/presentation-preview'

const DEFAULT_BLOB_MIME_TYPE = 'application/octet-stream'

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
        if (kind === 'pdf') return <PdfPreview data={data} onOpenExternally={handleOpenExternal} />
        if (kind === 'spreadsheet') return <SpreadsheetPreview data={data} onOpenExternally={handleOpenExternal} />
        if (kind === 'presentation') return <PresentationPreview data={data} onOpenExternally={handleOpenExternal} />
        return <HwpPreview data={data} onOpenExternally={handleOpenExternal} />
    }

    return <UnsupportedPreview fileName={fileName} onOpenExternal={handleOpenExternal} />
}
