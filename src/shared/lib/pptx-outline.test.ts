import { describe, expect, test } from 'bun:test'
import { parsePptxOutline } from '@shared/lib/pptx-outline'

const ZIP_LOCAL_FILE_HEADER_SIZE = 30
const ZIP_CENTRAL_DIRECTORY_HEADER_SIZE = 46
const ZIP_END_OF_CENTRAL_DIRECTORY_SIZE = 22

const textEncoder = new TextEncoder()

const slideXml = (paragraphs: string[]) =>
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">` +
    `<p:cSld><p:spTree><p:sp><p:txBody>` +
    paragraphs.map((text) => `<a:p><a:r><a:t>${text}</a:t></a:r></a:p>`).join('') +
    `</p:txBody></p:sp></p:spTree></p:cSld></p:sld>`

const emptySlideXml =
    `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
    `<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">` +
    `<p:cSld><p:spTree></p:spTree></p:cSld></p:sld>`

const buildStoredZip = (entries: { name: string; content: string }[]) => {
    const localChunks: Uint8Array[] = []
    const centralChunks: Uint8Array[] = []
    const localOffsets: number[] = []
    let cursor = 0

    for (const entry of entries) {
        const nameBytes = textEncoder.encode(entry.name)
        const contentBytes = textEncoder.encode(entry.content)
        const header = new DataView(new ArrayBuffer(ZIP_LOCAL_FILE_HEADER_SIZE))
        header.setUint32(0, 0x04034b50, true)
        header.setUint16(4, 20, true)
        header.setUint16(6, 0, true)
        header.setUint16(8, 0, true)
        header.setUint16(10, 0, true)
        header.setUint16(12, 0, true)
        header.setUint32(14, 0, true)
        header.setUint32(18, contentBytes.byteLength, true)
        header.setUint32(22, contentBytes.byteLength, true)
        header.setUint16(26, nameBytes.byteLength, true)
        header.setUint16(28, 0, true)

        localOffsets.push(cursor)
        localChunks.push(new Uint8Array(header.buffer), nameBytes, contentBytes)
        cursor += ZIP_LOCAL_FILE_HEADER_SIZE + nameBytes.byteLength + contentBytes.byteLength
    }

    entries.forEach((entry, entryIndex) => {
        const nameBytes = textEncoder.encode(entry.name)
        const contentBytes = textEncoder.encode(entry.content)
        const header = new DataView(new ArrayBuffer(ZIP_CENTRAL_DIRECTORY_HEADER_SIZE))
        header.setUint32(0, 0x02014b50, true)
        header.setUint16(4, 20, true)
        header.setUint16(6, 20, true)
        header.setUint16(8, 0, true)
        header.setUint16(10, 0, true)
        header.setUint16(12, 0, true)
        header.setUint16(14, 0, true)
        header.setUint32(16, 0, true)
        header.setUint32(20, contentBytes.byteLength, true)
        header.setUint32(24, contentBytes.byteLength, true)
        header.setUint16(28, nameBytes.byteLength, true)
        header.setUint16(30, 0, true)
        header.setUint16(32, 0, true)
        header.setUint16(34, 0, true)
        header.setUint16(36, 0, true)
        header.setUint32(38, 0, true)
        header.setUint32(42, localOffsets[entryIndex] ?? 0, true)
        centralChunks.push(new Uint8Array(header.buffer), nameBytes)
    })

    const centralDirectoryOffset = cursor
    const centralDirectorySize = centralChunks.reduce((sum, chunk) => sum + chunk.byteLength, 0)

    const eocd = new DataView(new ArrayBuffer(ZIP_END_OF_CENTRAL_DIRECTORY_SIZE))
    eocd.setUint32(0, 0x06054b50, true)
    eocd.setUint16(4, 0, true)
    eocd.setUint16(6, 0, true)
    eocd.setUint16(8, entries.length, true)
    eocd.setUint16(10, entries.length, true)
    eocd.setUint32(12, centralDirectorySize, true)
    eocd.setUint32(16, centralDirectoryOffset, true)
    eocd.setUint16(20, 0, true)

    const chunks = [...localChunks, ...centralChunks, new Uint8Array(eocd.buffer)]
    const totalSize = chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0)
    const result = new Uint8Array(totalSize)
    let writeOffset = 0
    for (const chunk of chunks) {
        result.set(chunk, writeOffset)
        writeOffset += chunk.byteLength
    }
    return result.buffer
}

describe('parsePptxOutline', () => {
    test('슬라이드 수와 슬라이드별 문단 텍스트를 파일명 순서대로 추출한다', async () => {
        const buffer = buildStoredZip([
            { name: '[Content_Types].xml', content: '<Types/>' },
            { name: 'ppt/slides/slide2.xml', content: slideXml(['두 번째 슬라이드']) },
            { name: 'ppt/slides/slide1.xml', content: slideXml(['제목', '첫 줄', '둘째 줄']) },
        ])

        const outline = await parsePptxOutline(buffer)

        expect(outline.slideCount).toBe(2)
        expect(outline.slides[0]).toEqual({ index: 1, paragraphs: ['제목', '첫 줄', '둘째 줄'] })
        expect(outline.slides[1]).toEqual({ index: 2, paragraphs: ['두 번째 슬라이드'] })
    })

    test('한 문단에 여러 <a:r> 런이 있으면 이어붙여 하나의 문단 텍스트로 만든다', async () => {
        const twoRunParagraphXml =
            `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>` +
            `<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">` +
            `<p:cSld><p:spTree><p:sp><p:txBody>` +
            `<a:p><a:r><a:t>안녕하세요, </a:t></a:r><a:r><a:t>TAIDE</a:t></a:r></a:p>` +
            `</p:txBody></p:sp></p:spTree></p:cSld></p:sld>`
        const buffer = buildStoredZip([{ name: 'ppt/slides/slide1.xml', content: twoRunParagraphXml }])

        const outline = await parsePptxOutline(buffer)

        expect(outline.slides[0]?.paragraphs).toEqual(['안녕하세요, TAIDE'])
    })

    test('텍스트가 없는 슬라이드는 빈 문단 배열을 반환한다', async () => {
        const buffer = buildStoredZip([{ name: 'ppt/slides/slide1.xml', content: emptySlideXml }])

        const outline = await parsePptxOutline(buffer)

        expect(outline.slideCount).toBe(1)
        expect(outline.slides[0]).toEqual({ index: 1, paragraphs: [] })
    })

    test('XML 엔티티(&amp; 등)를 원래 문자로 디코드한다', async () => {
        const buffer = buildStoredZip([{ name: 'ppt/slides/slide1.xml', content: slideXml(['A &amp; B &lt;tag&gt;']) }])

        const outline = await parsePptxOutline(buffer)

        expect(outline.slides[0]?.paragraphs).toEqual(['A & B <tag>'])
    })

    test('zip 형식이 아닌 손상된 입력은 에러를 던진다', async () => {
        const corrupted = new TextEncoder().encode('this is not a zip file at all').buffer

        await expect(parsePptxOutline(corrupted)).rejects.toThrow()
    })

    test('zip 이지만 central directory 시그니처가 깨진 입력은 에러를 던진다', async () => {
        const buffer = buildStoredZip([{ name: 'ppt/slides/slide1.xml', content: slideXml(['텍스트']) }])
        const corrupted = new Uint8Array(buffer)
        const centralDirectorySignatureOffset =
            corrupted.byteLength - ZIP_END_OF_CENTRAL_DIRECTORY_SIZE - ZIP_CENTRAL_DIRECTORY_HEADER_SIZE - 'ppt/slides/slide1.xml'.length
        corrupted[centralDirectorySignatureOffset] = 0x00

        await expect(parsePptxOutline(corrupted.buffer)).rejects.toThrow()
    })

    test('슬라이드가 하나도 없는 zip 은 에러를 던진다', async () => {
        const buffer = buildStoredZip([{ name: '[Content_Types].xml', content: '<Types/>' }])

        await expect(parsePptxOutline(buffer)).rejects.toThrow()
    })
})
