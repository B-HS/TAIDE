import { describe, expect, test } from 'bun:test'
import { validateEntryName } from '@shared/lib/entry-name'

describe('validateEntryName', () => {
    test('빈 문자열이면 null 을 반환한다', () => {
        expect(validateEntryName('', ['a.ts'])).toBeNull()
    })

    test('공백만 있으면 null 을 반환한다', () => {
        expect(validateEntryName('   ', ['a.ts'])).toBeNull()
    })

    test('. 은 예약된 이름이다', () => {
        expect(validateEntryName('.', [])).toBe('explorer.entryNameReserved')
    })

    test('.. 은 예약된 이름이다', () => {
        expect(validateEntryName('..', [])).toBe('explorer.entryNameReserved')
    })

    test('금지 문자가 있으면 invalidChar 를 반환한다', () => {
        expect(validateEntryName('a:b.ts', [])).toBe('explorer.entryNameInvalidChar')
        expect(validateEntryName('a<b>.ts', [])).toBe('explorer.entryNameInvalidChar')
    })

    test('끝이 마침표면 invalidChar 를 반환한다', () => {
        expect(validateEntryName('foo.', [])).toBe('explorer.entryNameInvalidChar')
    })

    test('같은 위치에 동명의 항목이 있으면 duplicate 를 반환한다', () => {
        expect(validateEntryName('a.ts', ['a.ts', 'b.ts'])).toBe('explorer.entryNameDuplicate')
    })

    test('통과하는 이름은 null 을 반환한다', () => {
        expect(validateEntryName('new-file.ts', ['a.ts'])).toBeNull()
    })

    test('중첩 경로는 마지막 세그먼트만 검증한다', () => {
        expect(validateEntryName('nested/dir/new-file.ts', ['a.ts'])).toBeNull()
        expect(validateEntryName('nested/dir/a.ts', ['a.ts'])).toBe('explorer.entryNameDuplicate')
    })

    test('앞뒤 공백은 trim 후 검증한다', () => {
        expect(validateEntryName('  foo.ts  ', ['foo.ts'])).toBe('explorer.entryNameDuplicate')
    })

    test('슬래시만 있으면 invalidChar 를 반환한다', () => {
        expect(validateEntryName('/', [])).toBe('explorer.entryNameInvalidChar')
    })
})
