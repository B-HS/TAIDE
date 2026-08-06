import { describe, expect, test } from 'bun:test'
import type { KeymapEntry, KeymapEvent } from '@shared/lib/keymap'
import { APP_KEYMAP, findMatchingKeymapEntry, matchesKeymapEntry } from '@shared/lib/keymap'

const baseEvent: KeymapEvent = { key: 'p', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }

describe('matchesKeymapEntry', () => {
    test('mac 에서 mod 는 metaKey 로 매칭된다', () => {
        const entry: KeymapEntry = { id: 'quick-open', key: 'p', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { ...baseEvent, metaKey: true }, true)).toBe(true)
        expect(matchesKeymapEntry(entry, { ...baseEvent, ctrlKey: true }, true)).toBe(false)
    })

    test('윈도우/리눅스에서 mod 는 ctrlKey 로 매칭된다', () => {
        const entry: KeymapEntry = { id: 'quick-open', key: 'p', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { ...baseEvent, ctrlKey: true }, false)).toBe(true)
        expect(matchesKeymapEntry(entry, { ...baseEvent, metaKey: true }, false)).toBe(false)
    })

    test('ctrl 수식키는 플랫폼과 무관하게 항상 물리 Ctrl 로만 매칭된다', () => {
        const entry: KeymapEntry = { id: 'git', key: 'g', mods: ['ctrl', 'shift'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: 'g', metaKey: false, ctrlKey: true, shiftKey: true, altKey: false }, true)).toBe(true)
        expect(matchesKeymapEntry(entry, { key: 'g', metaKey: true, ctrlKey: false, shiftKey: true, altKey: false }, true)).toBe(false)
        expect(matchesKeymapEntry(entry, { key: 'g', metaKey: false, ctrlKey: true, shiftKey: true, altKey: false }, false)).toBe(true)
    })

    test('수식키 조합(shift·alt)이 정확히 일치해야 한다', () => {
        const entry: KeymapEntry = { id: 'search', key: 'f', mods: ['mod', 'shift'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: 'f', metaKey: true, ctrlKey: false, shiftKey: true, altKey: false }, true)).toBe(true)
        expect(matchesKeymapEntry(entry, { key: 'f', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(false)
        expect(matchesKeymapEntry(entry, { key: 'f', metaKey: true, ctrlKey: false, shiftKey: true, altKey: true }, true)).toBe(false)
    })

    test('키 문자가 다르면 매칭되지 않는다', () => {
        const entry: KeymapEntry = { id: 'save', key: 's', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: 'a', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(false)
    })

    test('키 문자 대소문자를 구분하지 않는다', () => {
        const entry: KeymapEntry = { id: 'save', key: 's', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: 'S', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(true)
    })
})

describe('findMatchingKeymapEntry', () => {
    test('앱 전역 키맵에서 일치하는 항목을 찾는다', () => {
        const found = findMatchingKeymapEntry(APP_KEYMAP, { key: 'b', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found?.id).toBe('toggle-sidebar')
    })

    test('일치하는 항목이 없으면 null 을 반환한다', () => {
        const found = findMatchingKeymapEntry(APP_KEYMAP, { key: 'z', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found).toBeNull()
    })

    test('tab 순환 next/prev 는 shift 유무로 구분된다', () => {
        const next = findMatchingKeymapEntry(APP_KEYMAP, { key: 'Tab', metaKey: false, ctrlKey: true, shiftKey: false, altKey: false }, true)
        const prev = findMatchingKeymapEntry(APP_KEYMAP, { key: 'Tab', metaKey: false, ctrlKey: true, shiftKey: true, altKey: false }, true)
        expect(next?.id).toBe('tab-cycle-next')
        expect(prev?.id).toBe('tab-cycle-prev')
    })
})
