import { describe, expect, test } from 'bun:test'
import { resolvePreviewKind, resolvePreviewMimeType } from '@shared/lib/preview-kind'

describe('resolvePreviewKind', () => {
    test('이미지 확장자를 image 로 판정한다', () => {
        expect(resolvePreviewKind('photo.png')).toBe('image')
        expect(resolvePreviewKind('photo.jpg')).toBe('image')
        expect(resolvePreviewKind('photo.jpeg')).toBe('image')
        expect(resolvePreviewKind('photo.gif')).toBe('image')
        expect(resolvePreviewKind('photo.webp')).toBe('image')
        expect(resolvePreviewKind('photo.bmp')).toBe('image')
        expect(resolvePreviewKind('icon.svg')).toBe('image')
        expect(resolvePreviewKind('photo.avif')).toBe('image')
    })

    test('비디오 확장자를 video 로 판정한다', () => {
        expect(resolvePreviewKind('clip.mp4')).toBe('video')
        expect(resolvePreviewKind('clip.webm')).toBe('video')
        expect(resolvePreviewKind('clip.mov')).toBe('video')
        expect(resolvePreviewKind('clip.m4v')).toBe('video')
    })

    test('오디오 확장자를 audio 로 판정한다', () => {
        expect(resolvePreviewKind('song.mp3')).toBe('audio')
        expect(resolvePreviewKind('song.wav')).toBe('audio')
        expect(resolvePreviewKind('song.flac')).toBe('audio')
        expect(resolvePreviewKind('song.m4a')).toBe('audio')
        expect(resolvePreviewKind('song.ogg')).toBe('audio')
    })

    test('pdf 확장자를 pdf 로 판정한다', () => {
        expect(resolvePreviewKind('report.pdf')).toBe('pdf')
    })

    test('html 확장자를 html 로 판정한다', () => {
        expect(resolvePreviewKind('page.html')).toBe('html')
        expect(resolvePreviewKind('page.htm')).toBe('html')
    })

    test('스프레드시트 확장자를 spreadsheet 로 판정한다', () => {
        expect(resolvePreviewKind('sheet.xlsx')).toBe('spreadsheet')
        expect(resolvePreviewKind('sheet.xls')).toBe('spreadsheet')
        expect(resolvePreviewKind('sheet.csv')).toBe('spreadsheet')
    })

    test('pptx 확장자를 presentation 으로 판정한다', () => {
        expect(resolvePreviewKind('deck.pptx')).toBe('presentation')
    })

    test('hwp 확장자를 hwp 로 판정한다', () => {
        expect(resolvePreviewKind('doc.hwp')).toBe('hwp')
        expect(resolvePreviewKind('doc.hwpx')).toBe('hwp')
    })

    test('대소문자를 구분하지 않는다', () => {
        expect(resolvePreviewKind('PHOTO.PNG')).toBe('image')
        expect(resolvePreviewKind('Clip.Mp4')).toBe('video')
    })

    test('미지원 확장자는 null 을 반환한다', () => {
        expect(resolvePreviewKind('main.rs')).toBeNull()
        expect(resolvePreviewKind('index.ts')).toBeNull()
        expect(resolvePreviewKind('archive.zip')).toBeNull()
    })

    test('확장자가 없는 파일은 null 을 반환한다', () => {
        expect(resolvePreviewKind('README')).toBeNull()
        expect(resolvePreviewKind('Makefile')).toBeNull()
        expect(resolvePreviewKind('.gitignore')).toBeNull()
    })
})

describe('resolvePreviewMimeType', () => {
    test('이미지 확장자별 정확한 mime 타입을 반환한다', () => {
        expect(resolvePreviewMimeType('icon.svg')).toBe('image/svg+xml')
        expect(resolvePreviewMimeType('photo.png')).toBe('image/png')
        expect(resolvePreviewMimeType('photo.jpg')).toBe('image/jpeg')
    })

    test('html 은 text/html 을 반환한다', () => {
        expect(resolvePreviewMimeType('page.html')).toBe('text/html')
    })

    test('이미지·html 이 아니면 null 을 반환한다', () => {
        expect(resolvePreviewMimeType('clip.mp4')).toBeNull()
        expect(resolvePreviewMimeType('main.rs')).toBeNull()
    })
})
