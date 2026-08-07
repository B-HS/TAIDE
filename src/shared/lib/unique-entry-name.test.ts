import { describe, expect, test } from 'bun:test'
import { buildUniqueEntryName } from '@shared/lib/unique-entry-name'

describe('buildUniqueEntryName', () => {
    test('충돌이 없으면 원래 이름을 그대로 사용한다', () => {
        expect(buildUniqueEntryName('photo.png', ['other.png'], 'copy')).toBe('photo.png')
    })

    test('충돌하면 확장자 앞에 접미사를 붙인다', () => {
        expect(buildUniqueEntryName('photo.png', ['photo.png'], 'copy')).toBe('photo copy.png')
    })

    test('접미사도 충돌하면 번호를 증가시킨다', () => {
        expect(buildUniqueEntryName('photo.png', ['photo.png', 'photo copy.png'], 'copy')).toBe('photo copy 2.png')
        expect(buildUniqueEntryName('photo.png', ['photo.png', 'photo copy.png', 'photo copy 2.png'], 'copy')).toBe('photo copy 3.png')
    })

    test('확장자가 없는 이름도 처리한다', () => {
        expect(buildUniqueEntryName('notes', ['notes'], 'copy')).toBe('notes copy')
    })

    test('점으로 시작하는 이름은 전체를 base 로 취급한다', () => {
        expect(buildUniqueEntryName('.env', ['.env'], 'copy')).toBe('.env copy')
    })
})
