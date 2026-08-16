import { describe, expect, test } from 'bun:test'
import type { KeymapChordStage, KeymapEntry, KeymapEvent, KeymapOverrideEntry } from '@shared/lib/keymap'
import {
    APP_KEYMAP,
    MONACO_CHORD_PREFIX_KEY,
    applyKeymapOverrides,
    captureModsFromEvent,
    findKeymapConflict,
    findMatchingChordPrefixEntry,
    findMatchingKeymapEntry,
    formatKeymapShortcut,
    matchesChordSecondStage,
    matchesKeymapEntry,
    normalizeKeymapEventKey,
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

    test('key 가 빈 문자열(unbind 센티널)이면 항상 매칭되지 않는다', () => {
        const entry: KeymapEntry = { id: 'save', key: '', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: 's', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(false)
    })

    test('mac Option+Space 는 합성 문자(NBSP)로 도착해도 space 항목에 매칭된다', () => {
        const entry: KeymapEntry = { id: 'save', key: 'space', mods: ['alt'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: ' ', code: 'Space', metaKey: false, ctrlKey: false, shiftKey: false, altKey: true }, true)).toBe(true)
    })

    test('mac Option+문자는 합성 문자로 도착해도 물리 키 항목에 매칭된다', () => {
        const entry: KeymapEntry = { id: 'save', key: 'k', mods: ['alt'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: '˚', code: 'KeyK', metaKey: false, ctrlKey: false, shiftKey: false, altKey: true }, true)).toBe(true)
    })

    test("레거시 저장분 ' '(공백 한 글자)도 Space 이벤트에 매칭된다", () => {
        const entry: KeymapEntry = { id: 'save', key: ' ', mods: ['mod'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: ' ', code: 'Space', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(true)
    })

    test('레거시 스킴(구버전 event.key 원문)으로 저장된 mac Option 합성 문자도 물리 키 이벤트에 매칭된다', () => {
        const entry: KeymapEntry = { id: 'toggle-terminal', key: '˚', mods: ['alt'], descriptionKey: 'keymap.toggleTerminal' }
        expect(matchesKeymapEntry(entry, { key: '˚', code: 'KeyK', metaKey: false, ctrlKey: false, shiftKey: false, altKey: true }, true)).toBe(true)
    })

    test('레거시 스킴으로 저장된 mac Option+Space 의 NBSP 원문도 매칭된다', () => {
        const entry: KeymapEntry = { id: 'save', key: ' ', mods: ['alt'], descriptionKey: 'keymap.quickOpen' }
        expect(matchesKeymapEntry(entry, { key: ' ', code: 'Space', metaKey: false, ctrlKey: false, shiftKey: false, altKey: true }, true)).toBe(true)
    })

    test('신규 스킴(정규화된 물리 키)으로 저장된 항목은 그대로 매칭된다', () => {
        const entry: KeymapEntry = { id: 'toggle-terminal', key: 'k', mods: ['alt'], descriptionKey: 'keymap.toggleTerminal' }
        expect(matchesKeymapEntry(entry, { key: '˚', code: 'KeyK', metaKey: false, ctrlKey: false, shiftKey: false, altKey: true }, true)).toBe(true)
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

    test('폰트 크기 확대/축소는 mod+=/mod+- 로 매칭된다', () => {
        const up = findMatchingKeymapEntry(APP_KEYMAP, { key: '=', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        const down = findMatchingKeymapEntry(APP_KEYMAP, { key: '-', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(up?.id).toBe('font-size-up')
        expect(down?.id).toBe('font-size-down')
    })

    test('터미널 명령 간 이동은 mod+ArrowUp/ArrowDown 으로 매칭된다', () => {
        const previous = findMatchingKeymapEntry(APP_KEYMAP, { key: 'ArrowUp', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        const next = findMatchingKeymapEntry(APP_KEYMAP, { key: 'ArrowDown', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(previous?.id).toBe('terminal-jump-to-previous-command')
        expect(next?.id).toBe('terminal-jump-to-next-command')
    })

    test('Workspace Symbol 은 mod+t 로 매칭되고, 탭 재열기(mod+shift+t)와 shift 유무로 구분된다', () => {
        const workspaceSymbol = findMatchingKeymapEntry(APP_KEYMAP, { key: 't', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        const reopenClosedTab = findMatchingKeymapEntry(APP_KEYMAP, { key: 't', metaKey: true, ctrlKey: false, shiftKey: true, altKey: false }, true)
        expect(workspaceSymbol?.id).toBe('workspace-symbol')
        expect(reopenClosedTab?.id).toBe('reopen-closed-tab')
    })

    test('when 을 지정한 엔트리는 predicate 를 생략하면(기본값) 무조건 매칭된다 — 기존 21 엔트리 행동 변화 0', () => {
        const found = findMatchingKeymapEntry(APP_KEYMAP, { key: 'ArrowUp', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found?.id).toBe('terminal-jump-to-previous-command')
    })

    test('when predicate 가 false 면 when 을 가진 엔트리는 매칭에서 제외된다', () => {
        const found = findMatchingKeymapEntry(
            APP_KEYMAP,
            { key: 'ArrowUp', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false },
            true,
            () => false,
        )
        expect(found).toBeNull()
    })

    test('when 이 없는 엔트리는 predicate 가 false 여도 그대로 매칭된다', () => {
        const found = findMatchingKeymapEntry(
            APP_KEYMAP,
            { key: 'b', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false },
            true,
            () => false,
        )
        expect(found?.id).toBe('toggle-sidebar')
    })

    test('chord 를 가진 엔트리는 1단 키/mods 가 일치해도 제외된다', () => {
        const entries: KeymapEntry[] = [{ id: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] }, descriptionKey: 'keymap.save' }]
        const found = findMatchingKeymapEntry(entries, { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found).toBeNull()
    })
})

describe('findMatchingChordPrefixEntry', () => {
    const chordEntries: KeymapEntry[] = [
        { id: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] }, descriptionKey: 'keymap.save' },
        { id: 'close-tab', key: 'w', mods: ['mod'], descriptionKey: 'keymap.closeTab' },
    ]

    test('chord 를 가진 엔트리의 1단만 후보로 삼는다', () => {
        const found = findMatchingChordPrefixEntry(chordEntries, { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found?.id).toBe('save')
    })

    test('chord 가 없는 엔트리는 키가 일치해도 후보에서 제외된다', () => {
        const found = findMatchingChordPrefixEntry(chordEntries, { key: 'w', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(found).toBeNull()
    })

    test('isWhenSatisfied predicate 가 false 면 매칭되지 않는다(에디터 포커스 시 chord 1단 억제)', () => {
        const found = findMatchingChordPrefixEntry(
            chordEntries,
            { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false },
            true,
            () => false,
        )
        expect(found).toBeNull()
    })
})

describe('matchesChordSecondStage', () => {
    test('chord 의 2단 key/mods 와 일치하면 true 를 반환한다', () => {
        const entry: Pick<KeymapEntry, 'chord'> = { chord: { key: 's', mods: [] } }
        expect(matchesChordSecondStage(entry, { key: 's', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(true)
    })

    test('chord 의 2단과 일치하지 않으면 false 를 반환한다', () => {
        const entry: Pick<KeymapEntry, 'chord'> = { chord: { key: 's', mods: [] } }
        expect(matchesChordSecondStage(entry, { key: 'b', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(false)
    })

    test('chord 가 없으면 항상 false 를 반환한다', () => {
        const entry: Pick<KeymapEntry, 'chord'> = {}
        expect(matchesChordSecondStage(entry, { key: 's', metaKey: false, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(false)
    })
})

describe('MONACO_CHORD_PREFIX_KEY', () => {
    test('mac 에서 Cmd+K 로 매칭된다', () => {
        expect(matchesKeymapEntry(MONACO_CHORD_PREFIX_KEY, { key: 'k', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)).toBe(
            true,
        )
    })

    test('비-mac 에서 Ctrl+K 로 매칭된다', () => {
        expect(matchesKeymapEntry(MONACO_CHORD_PREFIX_KEY, { key: 'k', metaKey: false, ctrlKey: true, shiftKey: false, altKey: false }, false)).toBe(
            true,
        )
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

    test('구버전(하위호환) — chord 필드가 없는 저장분도 그대로 파싱된다', () => {
        const legacyJson = '[{"actionId":"save","key":"k","mods":["mod"]}]'
        expect(parseKeymapOverrides(legacyJson)).toEqual([{ actionId: 'save', key: 'k', mods: ['mod'] }])
    })

    test('신버전 — chord 필드를 가진 오버라이드를 파싱한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }]
        expect(parseKeymapOverrides(serializeKeymapOverrides(overrides))).toEqual(overrides)
    })

    test('전방호환 — chord 를 가진 신형 JSON 도 얕은 가드(actionId/key/mods)를 통과한다', () => {
        const newJson = '[{"actionId":"open-keybindings-editor","key":"k","mods":["mod"],"chord":{"key":"s","mods":[]}}]'
        const parsed = parseKeymapOverrides(newJson)
        expect(parsed).toHaveLength(1)
        expect(parsed[0]).toEqual({ actionId: 'open-keybindings-editor', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } })
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

    test('오버라이드가 chord 를 지정하면 base 엔트리에 chord 가 없어도 적용된다', () => {
        const result = applyKeymapOverrides(APP_KEYMAP, [{ actionId: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }])
        const saveEntry = result.find((entry) => entry.id === 'save')
        expect(saveEntry?.chord).toEqual({ key: 's', mods: [] })
    })

    test('chord 를 가진 base 엔트리를 chord 없는 오버라이드로 덮으면 단일 키로 다운그레이드된다', () => {
        const baseWithChord: KeymapEntry[] = [{ id: 'save', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] }, descriptionKey: 'keymap.save' }]
        const result = applyKeymapOverrides(baseWithChord, [{ actionId: 'save', key: 'x', mods: ['mod'] }])
        expect(result[0]?.chord).toBeUndefined()
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

    test('양쪽 when 이 모두 존재하고 다르면 key+mods 가 같아도 충돌로 보지 않는다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; when?: string }
        const rows: Row[] = [{ id: 'terminal.a', key: 'ArrowUp', mods: ['mod'], when: 'terminalFocus' }]
        const conflict = findKeymapConflict(rows, { key: 'ArrowUp', mods: ['mod'], when: 'editorTextFocus' }, 'editor.b', true)
        expect(conflict).toBeNull()
    })

    test('한쪽만 when 이 있으면 여전히 충돌로 본다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; when?: string }
        const rows: Row[] = [{ id: 'terminal.a', key: 'ArrowUp', mods: ['mod'], when: 'terminalFocus' }]
        const conflict = findKeymapConflict(rows, { key: 'ArrowUp', mods: ['mod'] }, 'unscoped.b', true)
        expect(conflict?.id).toBe('terminal.a')
    })

    test('양쪽 when 이 존재하고 동일하면 여전히 충돌로 본다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; when?: string }
        const rows: Row[] = [{ id: 'terminal.a', key: 'ArrowUp', mods: ['mod'], when: 'terminalFocus' }]
        const conflict = findKeymapConflict(rows, { key: 'ArrowUp', mods: ['mod'], when: 'terminalFocus' }, 'terminal.b', true)
        expect(conflict?.id).toBe('terminal.a')
    })

    test('APP_KEYMAP 의 터미널 점프 두 항목은 서로 다른 키라 충돌하지 않는다', () => {
        const conflict = findKeymapConflict(
            APP_KEYMAP,
            { key: 'ArrowUp', mods: ['mod'], when: 'terminalFocus' },
            'terminal-jump-to-next-command',
            true,
        )
        expect(conflict?.id).toBe('terminal-jump-to-previous-command')
    })

    test('같은 1단 프리픽스를 쓰는 두 chord 항목은 2단이 다르면 충돌로 보지 않는다 (⌘K 네임스페이스 공유)', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; chord?: KeymapChordStage }
        const rows: Row[] = [{ id: 'chord.a', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }]
        const conflict = findKeymapConflict(rows, { key: 'k', mods: ['mod'], chord: { key: 'x', mods: [] } }, 'chord.b', true)
        expect(conflict).toBeNull()
    })

    test('같은 1단 프리픽스에 2단까지 동일한 두 chord 항목은 충돌로 본다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; chord?: KeymapChordStage }
        const rows: Row[] = [{ id: 'chord.a', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }]
        const conflict = findKeymapConflict(rows, { key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }, 'chord.b', true)
        expect(conflict?.id).toBe('chord.a')
    })

    test('chord 항목과 같은 1단 키를 쓰는 단일-키(비 chord) 항목은 여전히 충돌로 본다 (chord 가 프리픽스 매칭을 선점해 단일 항목을 영구히 가린다)', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods']; chord?: KeymapChordStage }
        const rows: Row[] = [{ id: 'chord.a', key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }]
        const conflict = findKeymapConflict(rows, { key: 'k', mods: ['mod'] }, 'plain.b', true)
        expect(conflict?.id).toBe('chord.a')
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

describe('formatKeymapShortcut', () => {
    test('mac 에서는 심볼을 구분자 없이 연접한다(Apple 표기 순서 — mod(⌘) 는 항상 마지막)', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods'> = { key: 'p', mods: ['mod', 'shift'] }
        expect(formatKeymapShortcut(entry, true)).toBe('⇧⌘P')
    })

    test('mac 에서 ctrl 은 ⌃ 심볼로 표기된다', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods'> = { key: 'g', mods: ['ctrl', 'shift'] }
        expect(formatKeymapShortcut(entry, true)).toBe('⌃⇧G')
    })

    test('비-mac 에서는 + 로 구분된 단어로 표기된다', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods'> = { key: 'g', mods: ['ctrl', 'shift'] }
        expect(formatKeymapShortcut(entry, false)).toBe('Ctrl+Shift+G')
    })

    test('비-mac 에서 mod 와 ctrl 이 함께 있어도 Ctrl 이 중복 표기되지 않는다', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods'> = { key: 'g', mods: ['mod', 'ctrl'] }
        expect(formatKeymapShortcut(entry, false)).toBe('Ctrl+G')
    })

    test('단일 mod 조합은 플랫폼에 따라 다르게 표기된다', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods'> = { key: 's', mods: ['mod'] }
        expect(formatKeymapShortcut(entry, true)).toBe('⌘S')
        expect(formatKeymapShortcut(entry, false)).toBe('Ctrl+S')
    })

    test('chord 를 가진 엔트리는 두 단계를 공백으로 join 한다(mac)', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods' | 'chord'> = { key: 'k', mods: ['mod'], chord: { key: 's', mods: [] } }
        expect(formatKeymapShortcut(entry, true)).toBe('⌘K S')
    })

    test('chord 를 가진 엔트리는 두 단계를 공백으로 join 한다(비-mac, monaco defaultBindingLabel 표기와 동일)', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods' | 'chord'> = { key: 'k', mods: ['mod'], chord: { key: 's', mods: ['mod'] } }
        expect(formatKeymapShortcut(entry, false)).toBe('Ctrl+K Ctrl+S')
    })

    test('chord 가 없으면 1단만 표기한다(회귀 방지)', () => {
        const entry: Pick<KeymapEntry, 'key' | 'mods' | 'chord'> = { key: 's', mods: ['mod'] }
        expect(formatKeymapShortcut(entry, true)).toBe('⌘S')
    })
})

describe('findKeymapConflict 는 일반화된 행(id/key/mods) 목록에도 동작한다', () => {
    test('KeymapEntry 가 아닌 임의의 행 배열에서도 충돌을 찾는다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods'] }
        const rows: Row[] = [
            { id: 'command.a', key: 'k', mods: ['mod'] },
            { id: 'command.b', key: 'k', mods: ['mod'] },
        ]
        const conflict = findKeymapConflict(rows, { key: 'k', mods: ['mod'] }, 'command.a', true)
        expect(conflict?.id).toBe('command.b')
    })

    test('candidate 의 key 가 빈 문자열이면(unbind) 충돌로 보지 않는다', () => {
        type Row = { id: string; key: string; mods: KeymapEntry['mods'] }
        const rows: Row[] = [{ id: 'command.a', key: '', mods: [] }]
        const conflict = findKeymapConflict(rows, { key: '', mods: [] }, 'command.b', true)
        expect(conflict).toBeNull()
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

describe('normalizeKeymapEventKey', () => {
    test('mac Option+Space 의 NBSP(U+00A0)는 code 로 space 를 유도한다', () => {
        expect(normalizeKeymapEventKey({ key: ' ', code: 'Space' })).toBe('space')
    })

    test('일반 Space(공백 문자)도 canonical space 로 정규화한다', () => {
        expect(normalizeKeymapEventKey({ key: ' ', code: 'Space' })).toBe('space')
    })

    test('mac Option+문자의 합성 문자는 code 로 물리 키를 유도한다', () => {
        expect(normalizeKeymapEventKey({ key: '˚', code: 'KeyK' })).toBe('k')
        expect(normalizeKeymapEventKey({ key: '•', code: 'Digit8' })).toBe('8')
        expect(normalizeKeymapEventKey({ key: '–', code: 'Minus' })).toBe('-')
    })

    test('dead key(Option+E 등)는 code 로 물리 키를 유도한다', () => {
        expect(normalizeKeymapEventKey({ key: 'Dead', code: 'KeyE' })).toBe('e')
    })

    test('깨끗한 단일 문자 key 는 code 보다 우선한다(비 US 레이아웃 보존)', () => {
        expect(normalizeKeymapEventKey({ key: 'z', code: 'KeyY' })).toBe('z')
        expect(normalizeKeymapEventKey({ key: 'A', code: 'KeyA' })).toBe('a')
    })

    test('Enter·Tab·화살표·F키는 기존 key 경로를 유지한다', () => {
        expect(normalizeKeymapEventKey({ key: 'Enter', code: 'Enter' })).toBe('Enter')
        expect(normalizeKeymapEventKey({ key: 'ArrowUp', code: 'ArrowUp' })).toBe('ArrowUp')
        expect(normalizeKeymapEventKey({ key: 'F12', code: 'F12' })).toBe('F12')
    })

    test('code 가 없으면 key 정규화로 폴백한다', () => {
        expect(normalizeKeymapEventKey({ key: 'P' })).toBe('p')
        expect(normalizeKeymapEventKey({ key: 'Tab' })).toBe('Tab')
    })

    test('code 도 key 도 유도 불가하면 합성 문자를 그대로 소문자 정규화한다', () => {
        expect(normalizeKeymapEventKey({ key: 'ß', code: 'IntlBackslash' })).toBe('ß')
    })

    /**
     * Shift+symbol regression coverage (Wave H contract §3.5) — `event.key` for these arrives as
     * the shifted symbol itself (e.g. `'%'` for Shift+5), which `CLEAN_SINGLE_KEY_PATTERN` doesn't
     * match (it only accepts the *unshifted* punctuation/alnum set), so every one of these already
     * takes the `event.code` derivation path and resolves to the physical, unshifted key — the same
     * key a stored binding's `key` field records. Pins in code the fact the old gap analysis's
     * "shift+symbol partially supported" finding (docs/research §7) had already gone stale by the
     * time this was written (contract §2-1's confirmed-facts re-verification).
     */
    test.each([
        ['!', 'Digit1', '1'],
        ['@', 'Digit2', '2'],
        ['#', 'Digit3', '3'],
        ['$', 'Digit4', '4'],
        ['%', 'Digit5', '5'],
        ['^', 'Digit6', '6'],
        ['&', 'Digit7', '7'],
        ['*', 'Digit8', '8'],
        ['(', 'Digit9', '9'],
        [')', 'Digit0', '0'],
        ['_', 'Minus', '-'],
        ['+', 'Equal', '='],
        ['{', 'BracketLeft', '['],
        ['}', 'BracketRight', ']'],
        [':', 'Semicolon', ';'],
        ['"', 'Quote', "'"],
        ['~', 'Backquote', '`'],
        ['<', 'Comma', ','],
        ['>', 'Period', '.'],
        ['?', 'Slash', '/'],
        ['|', 'Backslash', '\\'],
    ])('shift+기호 %s(code=%s)는 물리 키 %s 로 유도된다', (key, code, expected) => {
        expect(normalizeKeymapEventKey({ key, code })).toBe(expected)
    })
})
