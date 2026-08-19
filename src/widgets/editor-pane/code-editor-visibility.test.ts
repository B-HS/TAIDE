import { describe, expect, test } from 'bun:test'
import type { monaco } from '@shared/lib/monaco/setup'
import { getEditorInstance, registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { canRenderCodeEditor } from '@widgets/editor-pane/code-editor-visibility'

describe('canRenderCodeEditor', () => {
    test('로딩·에러·refused 티어가 전부 아니면 CodeEditor 를 렌더한다', () => {
        expect(canRenderCodeEditor(false, false, 'normal')).toBe(true)
        expect(canRenderCodeEditor(false, false, undefined)).toBe(true)
    })

    test('isPending 이면(파일 쿼리 캐시 미스) CodeEditor 를 렌더하지 않는다', () => {
        expect(canRenderCodeEditor(true, false, undefined)).toBe(false)
    })

    test('isError 면 CodeEditor 를 렌더하지 않는다', () => {
        expect(canRenderCodeEditor(false, true, undefined)).toBe(false)
    })

    test("tier 가 'refused' 면 CodeEditor 를 렌더하지 않는다", () => {
        expect(canRenderCodeEditor(false, false, 'refused')).toBe(false)
    })
})

const createDisposableFakeEditor = () => {
    const state = { disposed: false }
    const editor = {
        getSupportedActions: () => {
            if (state.disposed) throw new Error('AbstractContextKeyService has been disposed')
            return []
        },
    } as unknown as monaco.editor.IStandaloneCodeEditor
    return { editor, dispose: () => (state.disposed = true) }
}

/**
 * Simulates one `EditorPane` commit's `[tabId, editor]` registration effect re-run (editor-pane.tsx
 * :171-175) — React tears down the previous commit's effect (unregistering the OLD tabId) before
 * running the new one (registering the NEW tabId with whatever `editor` value this commit
 * rendered). Uses the real `registerEditorInstance`/`unregisterEditorInstance` (not test doubles),
 * so this exercises the actual registry module the crash reaches through.
 */
const rerunRegistrationEffect = (previousTabId: string, nextTabId: string, editorForThisCommit: monaco.editor.IStandaloneCodeEditor | null) => {
    unregisterEditorInstance(previousTabId)
    if (editorForThisCommit) registerEditorInstance(nextTabId, editorForThisCommit)
}

/**
 * Reproduces the 2026-08-20 blank-window crash's root cause at the registry level
 * (docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md §1): a fresh-file click whose file
 * query is a cache miss deletes `CodeEditor` (disposing its monaco instance) in the same commit
 * that switches `tabId` — while `EditorPane`'s `editor` state still references that instance.
 * Without a render-phase adjustment, the pane's registration effect re-runs with the STALE,
 * about-to-be-disposed editor under the NEW tabId, planting a corpse `editor-area.tsx`'s
 * `getSupportedActions()` call later throws on.
 */
describe('캐시 미스 탭 전환에서 dispose 된 에디터 재등록 방지 (2026-08-20 빈 창 크래시 회귀)', () => {
    test('render-phase 조정으로 editor 상태를 null 화하면, 재등록 effect 가 dispose 대상 인스턴스를 새 tabId 로 등록하지 않는다', () => {
        const { editor: staleLiveEditorFromOldTab, dispose: disposeStaleEditor } = createDisposableFakeEditor()
        registerEditorInstance('tab-old', staleLiveEditorFromOldTab)

        const isCacheMissForNewTab = true
        disposeStaleEditor()
        const editorAfterRenderPhaseAdjustment = canRenderCodeEditor(isCacheMissForNewTab, false, undefined) ? staleLiveEditorFromOldTab : null

        rerunRegistrationEffect('tab-old', 'tab-new', editorAfterRenderPhaseAdjustment)

        expect(getEditorInstance('tab-new')).toBeNull()
        expect(getEditorInstance('tab-old')).toBeNull()
    })
})
