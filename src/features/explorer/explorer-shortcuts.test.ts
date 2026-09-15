import { describe, expect, test } from 'bun:test'
import type { ExplorerShortcutId } from '@features/explorer/explorer-shortcuts'
import { EXPLORER_SHORTCUT_LABELS, explorerShortcutBindingsOf, findExplorerShortcutId } from '@features/explorer/explorer-shortcuts'
import type { KeymapEvent } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, findMatchingKeymapEntry, formatKeymapShortcut, keymapEntryToEvent } from '@shared/lib/keymap/keymap'

const keydown = (key: string, mods: Partial<Pick<KeymapEvent, 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>> = {}): KeymapEvent => ({
    key,
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...mods,
})

const SHORTCUT_IDS = Object.keys(EXPLORER_SHORTCUT_LABELS) as ExplorerShortcutId[]

describe('findExplorerShortcutId', () => {
    test('Enter 와 F2 는 같은 이름 바꾸기 동작으로 해석된다', () => {
        expect(findExplorerShortcutId(keydown('Enter'), true)).toBe('rename')
        expect(findExplorerShortcutId(keydown('F2'), true)).toBe('rename')
    })

    test('수식키가 정확히 일치해야 한다 — ⌘C·⌥⌘C·⇧⌥⌘C 는 서로 가리지 않는다', () => {
        expect(findExplorerShortcutId(keydown('c', { metaKey: true }), true)).toBe('copy')
        expect(findExplorerShortcutId(keydown('c', { metaKey: true, altKey: true }), true)).toBe('copyPath')
        expect(findExplorerShortcutId(keydown('c', { metaKey: true, altKey: true, shiftKey: true }), true)).toBe('copyRelativePath')
    })

    test('Space 는 미리보기, ⌘↓ 는 고정 탭 열기다', () => {
        expect(findExplorerShortcutId(keydown(' '), true)).toBe('preview')
        expect(findExplorerShortcutId(keydown('ArrowDown', { metaKey: true }), true)).toBe('openPinned')
    })

    test('수식키 없는 방향키·글자는 트리 탐색과 타이프어헤드에 남겨 둔다', () => {
        expect(findExplorerShortcutId(keydown('ArrowDown'), true)).toBeNull()
        expect(findExplorerShortcutId(keydown('ArrowUp'), true)).toBeNull()
        expect(findExplorerShortcutId(keydown('c'), true)).toBeNull()
    })

    test('비 macOS 에서는 mod 가 Ctrl 로 풀린다', () => {
        expect(findExplorerShortcutId(keydown('c', { ctrlKey: true }), false)).toBe('copy')
        expect(findExplorerShortcutId(keydown('c', { metaKey: true }), false)).toBeNull()
    })
})

describe('탐색기 단축키 표', () => {
    test('모든 동작에 바인딩과 라벨이 함께 존재한다', () => {
        for (const id of SHORTCUT_IDS) {
            expect(explorerShortcutBindingsOf(id).length).toBeGreaterThan(0)
        }
    })

    test('라벨의 수식키 표기가 formatKeymapShortcut 와 어긋나지 않는다', () => {
        for (const id of SHORTCUT_IDS) {
            const [binding] = explorerShortcutBindingsOf(id)
            const modifierPrefix = formatKeymapShortcut({ key: '', mods: binding.mods }, true)
            expect(EXPLORER_SHORTCUT_LABELS[id].startsWith(modifierPrefix)).toBe(true)
        }
    })

    /**
     * The window capture listener runs before this local handler and swallows what it matches, so a
     * binding colliding with a `when`-less `APP_KEYMAP` entry would simply never fire in the tree.
     * The terminal's Cmd+Down is the one overlap and is allowed because its `when` excludes the tree.
     */
    test('when 없는 전역 키맵 엔트리와 겹치지 않는다', () => {
        const collisions = SHORTCUT_IDS.flatMap((id) =>
            explorerShortcutBindingsOf(id).flatMap((binding) => {
                const matched = findMatchingKeymapEntry(APP_KEYMAP, keymapEntryToEvent(binding, true), true)
                return matched && matched.when === undefined ? [`${id}/${matched.id}`] : []
            }),
        )

        expect(collisions).toEqual([])
    })
})
