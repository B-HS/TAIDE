import { describe, expect, test } from 'bun:test'
import type { AppCommand } from '@shared/lib/command-registry'
import type { KeymapOverrideEntry } from '@shared/lib/keymap/keymap'
import {
    buildKeybindingRows,
    buildUnbindOverride,
    filterKeybindingRowsByCapturedKey,
    findConflictingRow,
    findRunnableCommandBinding,
    mergeKeybindingOverride,
    removeKeybindingOverride,
    sortKeybindingRows,
} from '@shared/lib/keymap/keybinding-catalog'

const commands: AppCommand[] = [
    { id: 'editor.save', titleKey: 'keymap.save', categoryKey: 'keymap.category.editor', keymapId: 'save', run: () => {} },
    { id: 'view.toggleSidebar', titleKey: 'keymap.toggleSidebar', categoryKey: 'keymap.category.view', keymapId: 'toggle-sidebar', run: () => {} },
    { id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: 'keymap.category.window', run: () => {} },
    { id: 'settings.open', titleKey: 'settings.title', categoryKey: 'keymap.category.app', run: () => {} },
    {
        id: 'monaco.editor.action.triggerSuggest',
        titleKey: 'keymap.monaco.editor.action.triggerSuggest',
        titleDefaultValue: 'Trigger Suggest',
        categoryKey: 'keymap.category.editorSuggest',
        run: () => {},
    },
]

describe('buildKeybindingRows', () => {
    test('keymapId 가 있는 커맨드는 APP_KEYMAP 의 기본 바인딩을 그대로 가져온다', () => {
        const rows = buildKeybindingRows(commands, [])
        const saveRow = rows.find((row) => row.commandId === 'editor.save')
        expect(saveRow).toMatchObject({ id: 'save', key: 's', mods: ['mod'], isOverridden: false, runsViaCommand: false })
    })

    test('keymapId 가 없는 커맨드는 기본적으로 미할당(key: 빈 문자열)이며 커맨드 실행 경로로 표시된다', () => {
        const rows = buildKeybindingRows(commands, [])
        const reloadRow = rows.find((row) => row.commandId === 'window.reload')
        expect(reloadRow).toMatchObject({ id: 'window.reload', key: '', mods: [], runsViaCommand: true })
    })

    test('커맨드가 없는 키맵 액션도 행으로 포함되고 카테고리가 부여된다', () => {
        const rows = buildKeybindingRows(commands, [])
        const commandPaletteRow = rows.find((row) => row.id === 'command-palette')
        expect(commandPaletteRow).toMatchObject({ commandId: null, keymapId: 'command-palette', categoryKey: 'keymap.category.app' })
    })

    test('터미널 명령 간 이동 키맵 전용 액션도 터미널 카테고리가 부여된다', () => {
        const rows = buildKeybindingRows(commands, [])
        const prevRow = rows.find((row) => row.id === 'terminal-jump-to-previous-command')
        const nextRow = rows.find((row) => row.id === 'terminal-jump-to-next-command')
        expect(prevRow).toMatchObject({ commandId: null, categoryKey: 'keymap.category.terminal' })
        expect(nextRow).toMatchObject({ commandId: null, categoryKey: 'keymap.category.terminal' })
    })

    test('keymapId 가 있는 커맨드에 대응하는 APP_KEYMAP 항목은 중복 행으로 나타나지 않는다', () => {
        const rows = buildKeybindingRows(commands, [])
        const saveRows = rows.filter((row) => row.id === 'save')
        expect(saveRows).toHaveLength(1)
    })

    test('오버라이드가 있으면 key/mods 를 덮어쓰고 isOverridden 을 true 로 표시한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'r', mods: ['mod', 'shift'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.commandId === 'window.reload')
        expect(reloadRow).toMatchObject({ key: 'r', mods: ['mod', 'shift'], isOverridden: true })
    })

    test('unbind 센티널 오버라이드는 key 를 빈 문자열로 만든다', () => {
        const overrides = [buildUnbindOverride('save')]
        const rows = buildKeybindingRows(commands, overrides)
        const saveRow = rows.find((row) => row.id === 'save')
        expect(saveRow).toMatchObject({ key: '', mods: [], isOverridden: true })
    })
})

describe('findConflictingRow', () => {
    test('다른 행이 같은 물리 키 조합을 쓰면 그 행을 반환한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 's', mods: ['mod'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        const conflict = findConflictingRow(rows, reloadRow, true)
        expect(conflict?.id).toBe('save')
    })

    test('미할당 행은 충돌로 보지 않는다', () => {
        const rows = buildKeybindingRows(commands, [])
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        expect(findConflictingRow(rows, reloadRow, true)).toBeNull()
    })

    test('같은 1단 프리픽스(⌘K)를 쓰지만 2단이 다른 chord 로 재바인딩하면 기본 제공 chord 행(open-keybindings-editor, ⌘K ⌘S)과 충돌로 보지 않는다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'k', mods: ['mod'], chord: { key: 'x', mods: [] } }]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        expect(findConflictingRow(rows, reloadRow, true)).toBeNull()
    })

    test('같은 1단 프리픽스에 2단까지 동일한 chord 로 재바인딩하면 기본 제공 chord 행과 충돌로 본다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'k', mods: ['mod'], chord: { key: 's', mods: ['mod'] } }]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        expect(findConflictingRow(rows, reloadRow, true)?.id).toBe('open-keybindings-editor')
    })

    test('monaco 기본 바인딩(⌃Space)과 같은 키로 재바인딩하면 그 monaco 행을 충돌로 본다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'space', mods: ['ctrl'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        expect(findConflictingRow(rows, reloadRow, true)?.id).toBe('monaco.editor.action.triggerSuggest')
    })

    test('사용자가 해제(unbind)한 monaco 행은 기본 바인딩을 잃으므로 충돌로 보지 않는다', () => {
        const overrides: KeymapOverrideEntry[] = [
            { actionId: 'window.reload', key: 'space', mods: ['ctrl'] },
            buildUnbindOverride('monaco.editor.action.triggerSuggest'),
        ]
        const rows = buildKeybindingRows(commands, overrides)
        const reloadRow = rows.find((row) => row.id === 'window.reload')!
        expect(findConflictingRow(rows, reloadRow, true)).toBeNull()
    })

    test('monaco 행 자신도 기본 바인딩 기준으로 충돌 상대를 찾는다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'space', mods: ['ctrl'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const suggestRow = rows.find((row) => row.id === 'monaco.editor.action.triggerSuggest')!
        expect(findConflictingRow(rows, suggestRow, true)?.id).toBe('window.reload')
    })
})

describe('filterKeybindingRowsByCapturedKey', () => {
    test('캡처한 키 조합과 정확히 일치하는 행만 남긴다', () => {
        const rows = buildKeybindingRows(commands, [])
        const filtered = filterKeybindingRowsByCapturedKey(rows, 's', ['mod'], true)
        expect(filtered.map((row) => row.id)).toEqual(['save'])
    })

    test('미할당 행은 어떤 캡처에도 걸리지 않는다', () => {
        const rows = buildKeybindingRows(commands, [])
        const filtered = filterKeybindingRowsByCapturedKey(rows, 'z', [], true)
        expect(filtered).toEqual([])
    })
})

describe('sortKeybindingRows', () => {
    test('바인딩이 있는 행이 없는 행보다 앞에 오고, 그 안에서는 라벨 오름차순이다', () => {
        const rows = buildKeybindingRows(commands, [])
        const sorted = sortKeybindingRows(rows, (row) => row.titleKey)
        const firstUnassignedIndex = sorted.findIndex((row) => !row.key)
        const lastAssignedIndex = sorted.map((row) => !!row.key).lastIndexOf(true)
        expect(lastAssignedIndex).toBeLessThan(firstUnassignedIndex)
    })
})

describe('findRunnableCommandBinding', () => {
    test('keymapId 없는 커맨드에 사용자가 바인딩을 지정하면 매칭된다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'r', mods: ['mod', 'shift'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const match = findRunnableCommandBinding(rows, { key: 'r', metaKey: true, ctrlKey: false, shiftKey: true, altKey: false }, true)
        expect(match?.commandId).toBe('window.reload')
    })

    test('keymapId 가 있는 커맨드는 기존 액션 핸들러 경로를 쓰므로 이 함수로는 매칭되지 않는다(중복 실행 방지)', () => {
        const rows = buildKeybindingRows(commands, [])
        const match = findRunnableCommandBinding(rows, { key: 's', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(match).toBeNull()
    })

    test('chord 로 재바인딩된 행은 1단만으로 매칭되지 않는다(chord/유예 상태머신 밖 경로라 2단을 구현할 수 없음)', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'window.reload', key: 'r', mods: ['mod'], chord: { key: 's', mods: ['mod'] } }]
        const rows = buildKeybindingRows(commands, overrides)
        const match = findRunnableCommandBinding(rows, { key: 'r', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(match).toBeNull()
    })
})

describe('monaco 커맨드 행', () => {
    test('monaco. 접두 커맨드는 source: monaco 이고 카탈로그의 defaultBindingLabel 을 표시 전용으로 가져온다', () => {
        const rows = buildKeybindingRows(commands, [])
        const row = rows.find((r) => r.commandId === 'monaco.editor.action.triggerSuggest')
        expect(row).toMatchObject({ source: 'monaco', key: '', mods: [], defaultBindingLabel: '⌃Space', titleDefaultValue: 'Trigger Suggest' })
    })

    test('monaco 행은 오버라이드가 없어도 runsViaCommand 가 false 다 (전역 capture 가 실행하지 않는다)', () => {
        const rows = buildKeybindingRows(commands, [])
        const row = rows.find((r) => r.commandId === 'monaco.editor.action.triggerSuggest')
        expect(row?.runsViaCommand).toBe(false)
    })

    test('monaco 행에 오버라이드를 지정해도 findRunnableCommandBinding 은 매칭하지 않는다 (이중 실행 방지)', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'monaco.editor.action.triggerSuggest', key: 'i', mods: ['mod'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const match = findRunnableCommandBinding(rows, { key: 'i', metaKey: true, ctrlKey: false, shiftKey: false, altKey: false }, true)
        expect(match).toBeNull()
    })

    test('monaco 행에 오버라이드를 지정하면 key/mods 와 isOverridden 은 정상적으로 갱신된다 (표시·재바인딩 용도)', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'monaco.editor.action.triggerSuggest', key: 'i', mods: ['mod'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const row = rows.find((r) => r.commandId === 'monaco.editor.action.triggerSuggest')
        expect(row).toMatchObject({ key: 'i', mods: ['mod'], isOverridden: true })
    })
})

describe('chord 필드 전달', () => {
    test('커맨드에 대응하는 커맨드 없는 키맵 전용 행은 APP_KEYMAP 엔트리의 chord 를 그대로 가져온다', () => {
        const rows = buildKeybindingRows(commands, [])
        const openEditorRow = rows.find((row) => row.id === 'open-keybindings-editor')
        expect(openEditorRow?.chord).toEqual({ key: 's', mods: ['mod'] })
    })

    test('keymapId 가 chord 를 가진 APP_KEYMAP 엔트리를 가리키는 커맨드 행도 그 chord 를 가져온다', () => {
        const commandsWithChord: AppCommand[] = [
            { id: 'settings.openKeybindingsEditor', titleKey: 'settings.keymapOpenEditor', keymapId: 'open-keybindings-editor', run: () => {} },
        ]
        const rows = buildKeybindingRows(commandsWithChord, [])
        const row = rows.find((r) => r.commandId === 'settings.openKeybindingsEditor')
        expect(row?.chord).toEqual({ key: 's', mods: ['mod'] })
    })

    test('chord 를 가진 오버라이드는 base 엔트리의 chord 를 대체한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'save', key: 'k', mods: ['mod'], chord: { key: 'd', mods: ['shift'] } }]
        const rows = buildKeybindingRows(commands, overrides)
        const saveRow = rows.find((row) => row.id === 'save')
        expect(saveRow).toMatchObject({ key: 'k', mods: ['mod'], chord: { key: 'd', mods: ['shift'] }, isOverridden: true })
    })

    test('chord 없는 오버라이드는 base 엔트리의 chord 를 제거한다(단일 키로 다운그레이드)', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'open-keybindings-editor', key: 'j', mods: ['mod'] }]
        const rows = buildKeybindingRows(commands, overrides)
        const openEditorRow = rows.find((row) => row.id === 'open-keybindings-editor')
        expect(openEditorRow).toMatchObject({ key: 'j', mods: ['mod'], chord: undefined, isOverridden: true })
    })

    test('chord 가 없는 APP_KEYMAP 엔트리의 행은 chord 가 undefined 다', () => {
        const rows = buildKeybindingRows(commands, [])
        const saveRow = rows.find((row) => row.id === 'save')
        expect(saveRow?.chord).toBeUndefined()
    })
})

describe('mergeKeybindingOverride / removeKeybindingOverride', () => {
    test('같은 actionId 의 기존 오버라이드를 교체한다', () => {
        const overrides: KeymapOverrideEntry[] = [{ actionId: 'save', key: 'x', mods: [] }]
        const merged = mergeKeybindingOverride(overrides, { actionId: 'save', key: 'y', mods: ['mod'] })
        expect(merged).toEqual([{ actionId: 'save', key: 'y', mods: ['mod'] }])
    })

    test('actionId 로 오버라이드를 제거한다(기본값 복원)', () => {
        const overrides: KeymapOverrideEntry[] = [
            { actionId: 'save', key: 'y', mods: ['mod'] },
            { actionId: 'window.reload', key: 'r', mods: [] },
        ]
        expect(removeKeybindingOverride(overrides, 'save')).toEqual([{ actionId: 'window.reload', key: 'r', mods: [] }])
    })
})
