import { describe, expect, test } from 'bun:test'
import { resolveNumericFieldCommit } from '@shared/lib/numeric-field-commit'

const FIELD = { committedValue: 20, min: 1, max: 100 }

describe('resolveNumericFieldCommit — 설정 숫자 필드 blur (audit §4-B B15)', () => {
    test('범위를 넘긴 입력은 clamp 한 값으로 커밋한다(화면은 호출부가 실값으로 되돌린다)', () => {
        expect(resolveNumericFieldCommit({ ...FIELD, rawValue: 9999 })).toBe(100)
        expect(resolveNumericFieldCommit({ ...FIELD, rawValue: -5 })).toBe(1)
    })

    test('clamp 결과가 이미 저장된 값과 같으면 커밋하지 않는다(재현: 100 을 넘겨 쓴 뒤에도 9999 가 남던 자리)', () => {
        expect(resolveNumericFieldCommit({ ...FIELD, committedValue: 100, rawValue: 9999 })).toBeNull()
        expect(resolveNumericFieldCommit({ ...FIELD, rawValue: 20 })).toBeNull()
    })

    test('빈 입력(NaN)은 커밋하지 않는다', () => {
        expect(resolveNumericFieldCommit({ ...FIELD, rawValue: Number.NaN })).toBeNull()
    })

    test('범위 안의 새 값은 그대로 커밋한다', () => {
        expect(resolveNumericFieldCommit({ ...FIELD, rawValue: 42 })).toBe(42)
    })
})
