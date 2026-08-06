const ZIP_LOCAL_FILE_HEADER_SIGNATURE = 0x04034b50
const ZIP_CENTRAL_DIRECTORY_SIGNATURE = 0x02014b50
const ZIP_END_OF_CENTRAL_DIRECTORY_SIGNATURE = 0x06054b50

const ZIP_LOCAL_FILE_HEADER_SIZE = 30
const ZIP_CENTRAL_DIRECTORY_HEADER_SIZE = 46
const ZIP_END_OF_CENTRAL_DIRECTORY_SIZE = 22
const ZIP_END_OF_CENTRAL_DIRECTORY_MAX_COMMENT_LENGTH = 65535

const ZIP_COMPRESSION_STORED = 0
const ZIP_COMPRESSION_DEFLATE = 8

const SLIDE_ENTRY_PATTERN = /^ppt\/slides\/slide(\d+)\.xml$/
const PARAGRAPH_PATTERN = /<a:p>[\s\S]*?<\/a:p>/g
const TEXT_RUN_PATTERN = /<a:t>([\s\S]*?)<\/a:t>/g
const XML_ENTITY_PATTERN = /&(#x[0-9a-fA-F]+|#\d+|[a-z]+);/g
const XML_ENTITY_NAME_TO_CHAR: Record<string, string> = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" }

export type PptxSlideOutline = {
    index: number
    paragraphs: string[]
}

export type PptxOutline = {
    slideCount: number
    slides: PptxSlideOutline[]
}

type ZipEndOfCentralDirectory = {
    entryCount: number
    centralDirectoryOffset: number
}

type ZipSlideEntry = {
    slideNumber: number
    localHeaderOffset: number
    compressionMethod: number
    compressedSize: number
}

const findEndOfCentralDirectory = (view: DataView) => {
    const maxCommentLength = Math.min(view.byteLength - ZIP_END_OF_CENTRAL_DIRECTORY_SIZE, ZIP_END_OF_CENTRAL_DIRECTORY_MAX_COMMENT_LENGTH)

    for (let commentLength = 0; commentLength <= maxCommentLength; commentLength += 1) {
        const offset = view.byteLength - ZIP_END_OF_CENTRAL_DIRECTORY_SIZE - commentLength
        if (offset < 0) break
        if (view.getUint32(offset, true) !== ZIP_END_OF_CENTRAL_DIRECTORY_SIGNATURE) continue

        return {
            entryCount: view.getUint16(offset + 10, true),
            centralDirectoryOffset: view.getUint32(offset + 16, true),
        } satisfies ZipEndOfCentralDirectory
    }

    return null
}

const findSlideEntries = (view: DataView, bytes: Uint8Array, eocd: ZipEndOfCentralDirectory) => {
    const slideEntries: ZipSlideEntry[] = []
    const textDecoder = new TextDecoder('utf-8')
    let cursor = eocd.centralDirectoryOffset

    for (let entryIndex = 0; entryIndex < eocd.entryCount; entryIndex += 1) {
        if (view.getUint32(cursor, true) !== ZIP_CENTRAL_DIRECTORY_SIGNATURE) throw new Error('pptx-outline: corrupted central directory record')

        const compressionMethod = view.getUint16(cursor + 10, true)
        const compressedSize = view.getUint32(cursor + 20, true)
        const fileNameLength = view.getUint16(cursor + 28, true)
        const extraFieldLength = view.getUint16(cursor + 30, true)
        const fileCommentLength = view.getUint16(cursor + 32, true)
        const localHeaderOffset = view.getUint32(cursor + 42, true)

        const nameStart = cursor + ZIP_CENTRAL_DIRECTORY_HEADER_SIZE
        const fileName = textDecoder.decode(bytes.subarray(nameStart, nameStart + fileNameLength))
        const slideMatch = SLIDE_ENTRY_PATTERN.exec(fileName)
        if (slideMatch) slideEntries.push({ slideNumber: Number(slideMatch[1]), localHeaderOffset, compressionMethod, compressedSize })

        cursor += ZIP_CENTRAL_DIRECTORY_HEADER_SIZE + fileNameLength + extraFieldLength + fileCommentLength
    }

    return slideEntries
}

const inflateDeflateRaw = async (compressedBytes: Uint8Array) => {
    if (typeof DecompressionStream === 'undefined') throw new Error('pptx-outline: this runtime cannot decompress deflate zip entries')

    const stream = new Blob([compressedBytes.slice()]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
    return new Uint8Array(await new Response(stream).arrayBuffer())
}

const readSlideEntryBytes = async (view: DataView, bytes: Uint8Array, entry: ZipSlideEntry) => {
    if (view.getUint32(entry.localHeaderOffset, true) !== ZIP_LOCAL_FILE_HEADER_SIGNATURE)
        throw new Error('pptx-outline: corrupted local file header')

    const fileNameLength = view.getUint16(entry.localHeaderOffset + 26, true)
    const extraFieldLength = view.getUint16(entry.localHeaderOffset + 28, true)
    const dataStart = entry.localHeaderOffset + ZIP_LOCAL_FILE_HEADER_SIZE + fileNameLength + extraFieldLength
    const compressedBytes = bytes.subarray(dataStart, dataStart + entry.compressedSize)

    if (entry.compressionMethod === ZIP_COMPRESSION_STORED) return compressedBytes
    if (entry.compressionMethod === ZIP_COMPRESSION_DEFLATE) return inflateDeflateRaw(compressedBytes)
    throw new Error(`pptx-outline: unsupported zip compression method (${entry.compressionMethod})`)
}

const decodeXmlEntities = (value: string) =>
    value.replace(XML_ENTITY_PATTERN, (match, entity: string) => {
        if (entity.startsWith('#x')) return String.fromCodePoint(Number.parseInt(entity.slice(2), 16))
        if (entity.startsWith('#')) return String.fromCodePoint(Number.parseInt(entity.slice(1), 10))
        return XML_ENTITY_NAME_TO_CHAR[entity] ?? match
    })

/**
 * Extracts paragraph-level text from a DrawingML slide XML body.
 * Assumes the conventional `a:` namespace prefix that PowerPoint always emits —
 * this is an outline-level approximation, not a full OOXML parse.
 */
const extractParagraphs = (slideXml: string) =>
    (slideXml.match(PARAGRAPH_PATTERN) ?? [])
        .map((paragraphXml) => [...paragraphXml.matchAll(TEXT_RUN_PATTERN)].map((run) => decodeXmlEntities(run[1] ?? '')).join(''))
        .filter((paragraph) => paragraph.trim().length > 0)

/**
 * Parses a pptx (OOXML zip) file into an outline-level summary: per-slide
 * paragraph text only. This does not attempt to reproduce the original
 * layout, images, or formatting — see docs/features/preview.md §3.1.
 */
export const parsePptxOutline = async (buffer: ArrayBuffer) => {
    const bytes = new Uint8Array(buffer)
    const view = new DataView(buffer)

    const eocd = findEndOfCentralDirectory(view)
    if (!eocd) throw new Error('pptx-outline: not a valid zip archive')

    const slideEntries = findSlideEntries(view, bytes, eocd).toSorted((a, b) => a.slideNumber - b.slideNumber)
    if (slideEntries.length === 0) throw new Error('pptx-outline: no slides found in package')

    const slides = await Promise.all(
        slideEntries.map(async (entry, position) => {
            const slideBytes = await readSlideEntryBytes(view, bytes, entry)
            const slideXml = new TextDecoder('utf-8').decode(slideBytes)
            return { index: position + 1, paragraphs: extractParagraphs(slideXml) } satisfies PptxSlideOutline
        }),
    )

    return { slideCount: slides.length, slides }
}
