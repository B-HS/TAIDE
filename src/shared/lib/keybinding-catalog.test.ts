import { describe, expect, test } from 'bun:test'
import type { AppCommand } from '@shared/lib/command-registry'
import type { KeymapOverrideEntry } from '@shared/lib/keymap'
import {
    buildKeybindingRows,
    buildUnbindOverride,
    filterKeybindingRowsByCapturedKey,
    findConflictingRow,
    findRunnableCommandBinding,
    mergeKeybindingOverride,
    removeKeybindingOverride,
    sortKeybindingRows,
} from '@shared/lib/keybinding-catalog'

const commands: AppCommand[] = [
    { id: 'editor.save', titleKey: 'keymap.save', categoryKey: 'keymap.category.editor', keymapId: 'save', run: () => {} },
    { id: 'view.toggleSidebar', titleKey: 'keymap.toggleSidebar', categoryKey: 'keymap.category.view', keymapId: 'toggle-sidebar', run: () => {} },
    { id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: 'keymap.category.window', run: () => {} },
    { id: 'settings.open', titleKey: 'settings.title', categoryKey: 'keymap.category.app', run: () => {} },
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
