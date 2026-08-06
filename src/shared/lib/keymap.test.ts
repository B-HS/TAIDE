import { describe, expect, test } from 'bun:test'
import type { KeymapEntry, KeymapEvent, KeymapOverrideEntry } from '@shared/lib/keymap'
import {
    APP_KEYMAP,
    applyKeymapOverrides,
    captureModsFromEvent,
    findKeymapConflict,
    findMatchingKeymapEntry,
    matchesKeymapEntry,
    normalizeKeymapKey,
    parseKeymapOverrides,
    serializeKeymapOverrides,
} from '@shared/lib/keymap'

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

describe('parseKeymapOverrides', () => {
    test('null 이면 빈 배열을 반환한다', () => {
        expect(parseKeymapOverrides(null)).toEqual([])
    })

    test('빈 문자열이면 빈 배열을 반환한다', () => {
        expect(parseKeymapOverrides('')).toEqual([])
    })

    test('잘못된 JSON 이면 빈 배열을 반환한다', () => {
        expect(parseKeymapOverrides('{not json')).toEqual([])
    })

    test('배열이 아닌 JSON 이면 빈 배열을 반환한다', () => {
        expect(parseKeymapOverrides('{"actionId":"save"}')).toEqual([])
    })

    test('유효한 오버라이드 배열을 파싱한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'save', key: 'k', mods: ['mod', 'shift'] }]
        expect(parseKeymapOverrides(serializeKeymapOverrides(overrides))).toEqual(overrides)
    })

    test('형태가 어긋난 항목은 걸러낸다', () => {
        const parsed = parseKeymapOverrides('[{"actionId":"save","key":"k","mods":["mod"]},{"key":"x"}]')
        expect(parsed).toEqual([{ actionId: 'save', key: 'k', mods: ['mod'] }])
    })
})

describe('applyKeymapOverrides', () => {
    test('오버라이드가 없으면 기본 키맵을 그대로 반환한다', () => {
        const result = applyKeymapOverrides(APP_KEYMAP, [])
        expect(result).toEqual(APP_KEYMAP)
    })

    test('일치하는 actionId 의 key/mods 만 덮어쓴다', () => {
        const result = applyKeymapOverrides(APP_KEYMAP, [{ actionId: 'save', key: 'k', mods: ['mod', 'shift'] }])
        const saveEntry = result.find((entry) => entry.id === 'save')
        const closeTabEntry = result.find((entry) => entry.id === 'close-tab')
        expect(saveEntry).toEqual({ id: 'save', key: 'k', mods: ['mod', 'shift'], descriptionKey: 'keymap.save' })
        expect(closeTabEntry).toEqual(APP_KEYMAP.find((entry) => entry.id === 'close-tab'))
    })
})

describe('findKeymapConflict', () => {
    test('다른 액션이 같은 키 조합을 쓰면 그 항목을 반환한다', () => {
        const conflict = findKeymapConflict(APP_KEYMAP, { key: 'w', mods: ['mod'] }, 'save', true)
        expect(conflict?.id).toBe('close-tab')
    })

    test('자기 자신의 actionId 는 충돌로 보지 않는다', () => {
        const conflict = findKeymapConflict(APP_KEYMAP, { key: 'w', mods: ['mod'] }, 'close-tab', true)
        expect(conflict).toBeNull()
    })

    test('겹치는 항목이 없으면 null 을 반환한다', () => {
        const conflict = findKeymapConflict(APP_KEYMAP, { key: 'q', mods: ['mod', 'alt'] }, 'save', true)
        expect(conflict).toBeNull()
    })
})

describe('captureModsFromEvent', () => {
    test('mac 에서 metaKey 는 mod 로 캡처된다', () => {
        expect(captureModsFromEvent({ metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toEqual(['mod'])
    })

    test('mac 에서 물리 ctrlKey 는 ctrl 로 캡처된다', () => {
        expect(captureModsFromEvent({ metaKey: false, ctrlKey: true, shiftKey: false, altKey: false }, true)).toEqual(['ctrl'])
    })

    test('윈도우/리눅스에서 ctrlKey 는 mod 로 캡처된다', () => {
        expect(captureModsFromEvent({ metaKey: false, ctrlKey: true, shiftKey: false, altKey: false }, false)).toEqual(['mod'])
    })

    test('shift·alt 는 플랫폼과 무관하게 그대로 캡처된다', () => {
        expect(captureModsFromEvent({ metaKey: true, ctrlKey: false, shiftKey: true, altKey: true }, true)).toEqual(['mod', 'shift', 'alt'])
    })

    test('수식키가 없으면 빈 배열을 반환한다', () => {
        expect(captureModsFromEvent({ metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }, true)).toEqual([])
    })
})

describe('normalizeKeymapKey', () => {
    test('한 글자 키는 소문자로 정규화한다', () => {
        expect(normalizeKeymapKey('P')).toBe('p')
    })

    test('여러 글자 키는 그대로 유지한다', () => {
        expect(normalizeKeymapKey('Tab')).toBe('Tab')
        expect(normalizeKeymapKey('Escape')).toBe('Escape')
    })
})
