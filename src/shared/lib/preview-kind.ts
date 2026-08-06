export type PreviewKind = 'image' | 'video' | 'audio' | 'pdf' | 'html' | 'spreadsheet' | 'presentation' | 'hwp'

const PREVIEW_KIND_BY_EXTENSION: Record<string, PreviewKind> = {
    png: 'image',
    jpg: 'image',
    jpeg: 'image',
    gif: 'image',
    webp: 'image',
    bmp: 'image',
    svg: 'image',
    avif: 'image',
    mp4: 'video',
    webm: 'video',
    mov: 'video',
    m4v: 'video',
    mp3: 'audio',
    wav: 'audio',
    flac: 'audio',
    m4a: 'audio',
    ogg: 'audio',
    pdf: 'pdf',
    html: 'html',
    htm: 'html',
    xlsx: 'spreadsheet',
    xls: 'spreadsheet',
    csv: 'spreadsheet',
    pptx: 'presentation',
    hwp: 'hwp',
    hwpx: 'hwp',
}

const IMAGE_MIME_BY_EXTENSION: Record<string, string> = {
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    gif: 'image/gif',
    webp: 'image/webp',
    bmp: 'image/bmp',
    svg: 'image/svg+xml',
    avif: 'image/avif',
}

const DEFAULT_IMAGE_MIME_TYPE = 'application/octet-stream'

const extractExtension = (fileName: string) => {
    const dotIndex = fileName.lastIndexOf('.')
    return dotIndex <= 0 ? null : fileName.slice(dotIndex + 1).toLowerCase()
}

export const resolvePreviewKind = (fileName: string): PreviewKind | null => {
    const extension = extractExtension(fileName)
    return extension ? (PREVIEW_KIND_BY_EXTENSION[extension] ?? null) : null
}

export const resolvePreviewMimeType = (fileName: string): string | null => {
    const kind = resolvePreviewKind(fileName)
    if (kind === 'html') return 'text/html'
    if (kind !== 'image') return null

    const extension = extractExtension(fileName) ?? ''
    return IMAGE_MIME_BY_EXTENSION[extension] ?? DEFAULT_IMAGE_MIME_TYPE
}
