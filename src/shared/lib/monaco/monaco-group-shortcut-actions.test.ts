import { describe, expect, test } from 'bun:test'
import type { KeymapActionId, KeymapEntry, KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import { APP_KEYMAP, applyKeymapOverrides, formatKeymapShortcut } from '@shared/lib/keymap/keymap'
import { MONACO_ACTIONS } from '@shared/lib/monaco/monaco-actions'
import { buildMonacoChordKeybinding } from '@shared/lib/monaco/monaco-keybinding'
import {
    EDITOR_GROUP_SHORTCUT_ACTION_ID_PREFIX,
    buildEditorGroupShortcutActions,
    isEditorGroupShortcutKeymapId,
} from '@shared/lib/monaco/monaco-group-shortcut-actions'

const EXPECTED_KEYMAP_IDS: KeymapActionId[] = [
    'focus-group-left',
    'focus-group-right',
    'focus-group-up',
    'focus-group-down',
    'move-tab-to-group-left',
    'move-tab-to-group-right',
    'close-all-tabs',
]

const actionsFromAppKeymap = () => buildEditorGroupShortcutActions(APP_KEYMAP)

const appKeymapEntry = (keymapId: string) => {
    const entry = APP_KEYMAP.find((candidate) => candidate.id === keymapId)
    if (!entry) throw new Error(`APP_KEYMAP 에 ${keymapId} 엔트리가 없다`)
    return entry
}

const chordOf = (entry: KeymapEntry) => {
    if (!entry.chord) throw new Error(`${entry.id} 는 chord 엔트리가 아니다`)
    return entry.chord
}

const MONACO_CHORD_PREFIX_LABEL = '⌘K '

describe('buildEditorGroupShortcutActions — 등록 대상', () => {
    test('APP_KEYMAP 에서 ⌘K chord 7건만 골라낸다(⌘1~9 단일 키는 제외)', () => {
        expect(actionsFromAppKeymap().map((action) => action.keymapId)).toEqual(EXPECTED_KEYMAP_IDS)
    })

    test('멤버십 판정이 7건만 참이고 나머지 APP_KEYMAP 엔트리에는 거짓이다', () => {
        expect(APP_KEYMAP.filter((entry) => isEditorGroupShortcutKeymapId(entry.id)).map((entry) => entry.id)).toEqual(EXPECTED_KEYMAP_IDS)
        expect(isEditorGroupShortcutKeymapId('focus-group-1')).toBe(false)
        expect(isEditorGroupShortcutKeymapId('tab-cycle-next')).toBe(false)
    })

    test('action id 는 taide. 접두사 + 키맵 id 다', () => {
        expect(actionsFromAppKeymap().map((action) => action.actionId)).toEqual(EXPECTED_KEYMAP_IDS.map((id) => `taide.${id}`))
        expect(EDITOR_GROUP_SHORTCUT_ACTION_ID_PREFIX).toBe('taide.')
    })

    test('라벨 키는 미러링하는 키맵 엔트리의 descriptionKey 를 그대로 쓴다', () => {
        for (const action of actionsFromAppKeymap()) {
            expect(action.labelKey).toBe(appKeymapEntry(action.keymapId).descriptionKey)
        }
    })

    test('각 액션은 자기 키맵 엔트리에 대응하는 EditorPaneCommand 를 싣는다', () => {
        const commandsByKeymapId = Object.fromEntries(actionsFromAppKeymap().map((action) => [action.keymapId, action.command]))
        expect(commandsByKeymapId['focus-group-left']).toEqual({ type: 'focus-group', target: { kind: 'direction', direction: 'left' } })
        expect(commandsByKeymapId['focus-group-down']).toEqual({ type: 'focus-group', target: { kind: 'direction', direction: 'down' } })
        expect(commandsByKeymapId['move-tab-to-group-right']).toEqual({ type: 'move-tab-to-group', direction: 'right' })
        expect(commandsByKeymapId['close-all-tabs']).toEqual({ type: 'close-all-tabs' })
    })
})

describe('buildEditorGroupShortcutActions — 키코드', () => {
    test('기본 바인딩은 monaco 2단 chord 인코딩 값으로 변환된다', () => {
        for (const action of actionsFromAppKeymap()) {
            const entry = appKeymapEntry(action.keymapId)
            expect(action.keybindings).toEqual([buildMonacoChordKeybinding(entry, chordOf(entry)) ?? 0])
        }
    })

    test('⌘K ⌘← 는 1단 저 16비트 · 2단 고 16비트로 패킹된다', () => {
        const focusLeft = actionsFromAppKeymap().find((action) => action.keymapId === 'focus-group-left')
        const CTRL_CMD = 2048
        const SHIFT = 1024
        const KEY_K = 41
        const ARROW_LEFT = 15
        expect(focusLeft?.keybindings).toEqual([CTRL_CMD | KEY_K | ((CTRL_CMD | ARROW_LEFT) << 16)])

        const moveLeft = actionsFromAppKeymap().find((action) => action.keymapId === 'move-tab-to-group-left')
        expect(moveLeft?.keybindings).toEqual([CTRL_CMD | KEY_K | ((CTRL_CMD | SHIFT | ARROW_LEFT) << 16)])
    })

    test('오버라이드로 chord 를 다시 묶으면 재바인딩된 키로 등록된다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'focus-group-left', key: 'j', mods: ['mod'], chord: { key: 'h', mods: ['mod'] } }]
        const action = buildEditorGroupShortcutActions(applyKeymapOverrides(APP_KEYMAP, overrides)).find(
            (candidate) => candidate.keymapId === 'focus-group-left',
        )
        const CTRL_CMD = 2048
        const KEY_J = 40
        const KEY_H = 38
        expect(action?.keybindings).toEqual([CTRL_CMD | KEY_J | ((CTRL_CMD | KEY_H) << 16)])
    })

    test('오버라이드가 chord 를 없애면 키 없이 액션만 등록한다(앱 디스패치가 그 키의 단독 주인이 된다)', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'focus-group-left', key: 'ArrowLeft', mods: ['mod', 'alt'] }]
        const action = buildEditorGroupShortcutActions(applyKeymapOverrides(APP_KEYMAP, overrides)).find(
            (candidate) => candidate.keymapId === 'focus-group-left',
        )
        expect(action?.keybindings).toEqual([])
    })

    test('monaco KeyCode 가 없는 키로 재바인딩되면 키 없이 액션만 등록한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'close-all-tabs', key: 'k', mods: ['mod'], chord: { key: 'Unknown', mods: [] } }]
        const action = buildEditorGroupShortcutActions(applyKeymapOverrides(APP_KEYMAP, overrides)).find(
            (candidate) => candidate.keymapId === 'close-all-tabs',
        )
        expect(action?.keybindings).toEqual([])
    })
})

describe('buildEditorGroupShortcutActions — monaco 내장 ⌘K 네임스페이스와 무충돌', () => {
    const monacoChordSecondStages = MONACO_ACTIONS.flatMap((entry) =>
        entry.defaultBindingLabel?.startsWith(MONACO_CHORD_PREFIX_LABEL) ? [entry.defaultBindingLabel.slice(MONACO_CHORD_PREFIX_LABEL.length)] : [],
    )

    test('비교 대상인 monaco 내장 ⌘K chord 가 실제로 존재한다(오라클 공회전 방지)', () => {
        expect(monacoChordSecondStages.length).toBe(21)
    })

    test('7개 chord 의 2단이 monaco 내장 ⌘K chord 2단과 하나도 겹치지 않는다', () => {
        for (const action of actionsFromAppKeymap()) {
            const secondStageLabel = formatKeymapShortcut(appKeymapEntry(action.keymapId), true).slice(MONACO_CHORD_PREFIX_LABEL.length)
            expect(monacoChordSecondStages).not.toContain(secondStageLabel)
        }
    })

    test('7개 chord 는 전부 ⌘K 프리픽스를 쓴다(2단 비교가 같은 네임스페이스 안에서 이뤄진다)', () => {
        for (const action of actionsFromAppKeymap()) {
            expect(formatKeymapShortcut(appKeymapEntry(action.keymapId), true).startsWith(MONACO_CHORD_PREFIX_LABEL)).toBe(true)
        }
    })
})
