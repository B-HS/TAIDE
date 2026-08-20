import { describe, expect, test } from 'bun:test'
import { extractFileExtension } from '@shared/lib/file-extension'

describe('extractFileExtension', () => {
    test('확장자를 소문자로 반환한다', () => {
        expect(extractFileExtension('photo.PNG')).toBe('png')
        expect(extractFileExtension('a.TSX')).toBe('tsx')
    })

    test('확장자가 없는 파일은 null 을 반환한다', () => {
        expect(extractFileExtension('README')).toBeNull()
        expect(extractFileExtension('Makefile')).toBeNull()
    })

    test('점으로 시작하는 닷파일은 확장자 없음으로 취급한다', () => {
        expect(extractFileExtension('.gitignore')).toBeNull()
    })

    test('마지막 점 기준으로 확장자를 추출한다', () => {
        expect(extractFileExtension('archive.tar.gz')).toBe('gz')
    })
})
