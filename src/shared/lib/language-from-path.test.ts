import { describe, expect, test } from 'bun:test'
import { getLanguageIdFromPath } from '@shared/lib/language-from-path'

describe('getLanguageIdFromPath', () => {
    test('알려진 확장자는 매핑된 language id 를 반환한다', () => {
        expect(getLanguageIdFromPath('src/main.rs')).toBe('rust')
        expect(getLanguageIdFromPath('a.tsx')).toBe('typescriptreact')
        expect(getLanguageIdFromPath('a.TSX')).toBe('typescriptreact')
        expect(getLanguageIdFromPath('config.toml')).toBe('toml')
    })

    test('알 수 없는 확장자는 plaintext 로 폴백한다', () => {
        expect(getLanguageIdFromPath('a.unknown')).toBe('plaintext')
    })

    test('확장자가 없는 파일은 plaintext 로 폴백한다', () => {
        expect(getLanguageIdFromPath('Makefile')).toBe('plaintext')
    })

    test('닷파일(.gitignore)은 확장자 없이 plaintext 로 폴백한다', () => {
        expect(getLanguageIdFromPath('.gitignore')).toBe('plaintext')
    })

    test('경로 중 파일명만으로 판단한다', () => {
        expect(getLanguageIdFromPath('a/b/c/main.py')).toBe('python')
    })
})
