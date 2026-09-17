import { describe, expect, test } from 'bun:test'
import type { monaco } from '@shared/lib/monaco/setup'
import { registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { consumePendingReveal, revealInTab } from '@entities/editor/reveal-registry'

/**
 * Imported statically, unlike the path-keyed predecessor this file used to test: `reveal-registry`
 * no longer reaches `@shared/lib/monaco/setup` at runtime (it keyed reveals by walking
 * `monaco.editor.getEditors()`, which is exactly the "any editor with this path" lookup audit #9
 * removed), so there are no `?worker` bundles left in its import graph for `bun test` to choke on
 * and no `mock.module` workaround to keep.
 */
type FakeEditor = {
    positions: { lineNumber: number; column: number }[]
    revealed: { lineNumber: number; column: number }[]
    focusCount: number
    setPosition: (position: { lineNumber: number; column: number }) => void
    revealPositionInCenter: (position: { lineNumber: number; column: number }) => void
    focus: () => void
}

const createFakeEditor = (): FakeEditor => {
    const editor: FakeEditor = {
        positions: [],
        revealed: [],
        focusCount: 0,
        setPosition: (position) => editor.positions.push(position),
        revealPositionInCenter: (position) => editor.revealed.push(position),
        focus: () => {
            editor.focusCount += 1
        },
    }
    return editor
}

const asEditor = (editor: FakeEditor) => editor as unknown as monaco.editor.IStandaloneCodeEditor

const SHORT_TTL_MS = 20

describe('revealInTab / 대상 탭의 에디터가 이미 마운트된 경우', () => {
    test('그 에디터의 커서를 즉시 옮기고 보류를 남기지 않는다', () => {
        const editor = createFakeEditor()
        registerEditorInstance('tab-mounted', asEditor(editor))

        revealInTab('tab-mounted', { line: 3, column: 5 })

        expect(editor.positions).toEqual([{ lineNumber: 3, column: 5 }])
        expect(editor.revealed).toEqual([{ lineNumber: 3, column: 5 }])
        expect(editor.focusCount).toBe(1)

        const remounted = createFakeEditor()
        consumePendingReveal('tab-mounted', asEditor(remounted))
        expect(remounted.positions).toEqual([])

        unregisterEditorInstance('tab-mounted')
    })

    test('같은 파일을 열고 있는 다른 탭의 에디터는 건드리지 않는다', () => {
        const otherPaneEditor = createFakeEditor()
        const targetEditor = createFakeEditor()
        registerEditorInstance('tab-same-file-other-pane', asEditor(otherPaneEditor))
        registerEditorInstance('tab-same-file-target', asEditor(targetEditor))

        revealInTab('tab-same-file-target', { line: 120, column: 7 })

        expect(targetEditor.positions).toEqual([{ lineNumber: 120, column: 7 }])
        expect(otherPaneEditor.positions).toEqual([])
        expect(otherPaneEditor.focusCount).toBe(0)

        unregisterEditorInstance('tab-same-file-other-pane')
        unregisterEditorInstance('tab-same-file-target')
    })
})

describe('revealInTab / 대상 탭이 아직 마운트되지 않은 경우', () => {
    test('그 탭이 마운트되면 보류된 위치를 적용한다', () => {
        revealInTab('tab-not-yet-mounted', { line: 10, column: 2 })

        const editor = createFakeEditor()
        consumePendingReveal('tab-not-yet-mounted', asEditor(editor))

        expect(editor.positions).toEqual([{ lineNumber: 10, column: 2 }])
    })

    test('보류는 그 탭에만 붙어 다른 탭이 먼저 마운트돼도 소비되지 않는다', () => {
        revealInTab('tab-pending-owner', { line: 42, column: 1 })

        const strangerEditor = createFakeEditor()
        consumePendingReveal('tab-pending-stranger', asEditor(strangerEditor))
        expect(strangerEditor.positions).toEqual([])

        const ownerEditor = createFakeEditor()
        consumePendingReveal('tab-pending-owner', asEditor(ownerEditor))
        expect(ownerEditor.positions).toEqual([{ lineNumber: 42, column: 1 }])
    })

    test('같은 탭을 다시 요청하면 가장 최근 요청의 위치만 적용된다', () => {
        revealInTab('tab-re-requested', { line: 1, column: 1 })
        revealInTab('tab-re-requested', { line: 99, column: 4 })

        const editor = createFakeEditor()
        consumePendingReveal('tab-re-requested', asEditor(editor))

        expect(editor.positions).toEqual([{ lineNumber: 99, column: 4 }])
    })

    test('한 번 소비한 보류는 다음 마운트에 다시 적용되지 않는다', () => {
        revealInTab('tab-consumed-once', { line: 8, column: 3 })

        const firstMount = createFakeEditor()
        consumePendingReveal('tab-consumed-once', asEditor(firstMount))
        const secondMount = createFakeEditor()
        consumePendingReveal('tab-consumed-once', asEditor(secondMount))

        expect(firstMount.positions).toEqual([{ lineNumber: 8, column: 3 }])
        expect(secondMount.positions).toEqual([])
    })

    test('TTL 이 지나면 보류된 요청이 폐기되어 나중에 마운트돼도 커서를 옮기지 않는다', async () => {
        revealInTab('tab-expired', { line: 7, column: 1 }, SHORT_TTL_MS)
        await new Promise((resolve) => setTimeout(resolve, SHORT_TTL_MS * 3))

        const editor = createFakeEditor()
        consumePendingReveal('tab-expired', asEditor(editor))

        expect(editor.positions).toEqual([])
    })

    test('만료 전에는 정상적으로 적용된다', () => {
        revealInTab('tab-not-expired', { line: 7, column: 1 }, SHORT_TTL_MS * 10)

        const editor = createFakeEditor()
        consumePendingReveal('tab-not-expired', asEditor(editor))

        expect(editor.positions).toEqual([{ lineNumber: 7, column: 1 }])
    })
})

describe('consumePendingReveal / 보류된 요청이 없는 경우', () => {
    test('아무 것도 하지 않는다', () => {
        const editor = createFakeEditor()

        consumePendingReveal('tab-none', asEditor(editor))

        expect(editor.positions).toEqual([])
        expect(editor.focusCount).toBe(0)
    })
})
