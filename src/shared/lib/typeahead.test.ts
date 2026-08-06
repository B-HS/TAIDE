import { describe, expect, test } from 'bun:test'
import { findTypeaheadMatchIndex } from '@shared/lib/typeahead'

const items = [{ name: 'App.tsx' }, { name: 'button.tsx' }, { name: 'card.tsx' }, { name: 'Cn.ts' }]

describe('findTypeaheadMatchIndex', () => {
    test('현재 위치 다음부터 접두사가 일치하는 항목을 찾는다', () => {
        expect(findTypeaheadMatchIndex(items, 'c', 0)).toBe(2)
    })

    test('대소문자를 구분하지 않는다', () => {
        expect(findTypeaheadMatchIndex(items, 'CARD', 0)).toBe(2)
    })

    test('끝까지 못 찾으면 처음으로 돌아가 순환한다', () => {
        expect(findTypeaheadMatchIndex(items, 'a', 1)).toBe(0)
    })

    test('일치하는 항목이 없으면 -1 을 반환한다', () => {
        expect(findTypeaheadMatchIndex(items, 'z', 0)).toBe(-1)
    })

    test('버퍼가 비어있으면 -1 을 반환한다', () => {
        expect(findTypeaheadMatchIndex(items, '', 0)).toBe(-1)
    })

    test('같은 접두사가 여러 개면 현재 위치 바로 다음 것을 우선한다', () => {
        expect(findTypeaheadMatchIndex(items, 'c', 2)).toBe(3)
    })
})
