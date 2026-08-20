import { describe, expect, test } from 'bun:test'
import type { monaco } from '@shared/lib/monaco/setup'
import { getEditorInstance, registerEditorInstance, unregisterEditorInstance } from '@entities/editor/editor-instance-registry'
import { OPEN_FILE_HISTORY_MONACO_ACTION_ID } from '@entities/git/git.constant'
import { canRenderCodeEditor, resolveEditorStateForRender } from '@widgets/editor-pane/code-editor-visibility'

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

    test('tier 가 null 이어도 undefined 와 동일하게 취급한다 (useEditorLspIntegration 의 호출 형태)', () => {
        expect(canRenderCodeEditor(false, false, null)).toBe(true)
    })
})

describe('resolveEditorStateForRender', () => {
    test('canRenderCodeEditor 가 true 면 전달받은 editor 를 그대로 반환한다', () => {
        const editor = {} as monaco.editor.IStandaloneCodeEditor
        expect(resolveEditorStateForRender(editor, false, false, 'normal')).toBe(editor)
    })

    test('canRenderCodeEditor 가 false 면 editor 가 살아 있어도 null 로 조정한다', () => {
        const editor = {} as monaco.editor.IStandaloneCodeEditor
        expect(resolveEditorStateForRender(editor, true, false, undefined)).toBeNull()
        expect(resolveEditorStateForRender(editor, false, true, undefined)).toBeNull()
        expect(resolveEditorStateForRender(editor, false, false, 'refused')).toBeNull()
    })

    test('editor 가 이미 null 이면 canRenderCodeEditor 값과 무관하게 null 을 유지한다', () => {
        expect(resolveEditorStateForRender(null, false, false, 'normal')).toBeNull()
    })
})

/**
 * A registry-identity placeholder for the "stale, already-disposed" editor value the render-phase
 * adjustment and registration effect route around. The test below only checks which `tabId` ends
 * up holding a reference in `editor-instance-registry` — it never calls a method on the value
 * itself — so unlike {@link createRechargeableFakeEditor} below, this does not need to model
 * monaco's action-map bookkeeping (an earlier revision gave it a `getSupportedActions` that threw
 * immediately on dispose, contradicting `createRechargeableFakeEditor`'s dispose-alone-is-harmless
 * model below without ever actually being exercised by an assertion; removed).
 */
const createDisposableFakeEditor = () => {
    const editor = {} as unknown as monaco.editor.IStandaloneCodeEditor
    return { editor, dispose: () => undefined }
}

/**
 * Simulates one `EditorPane` commit's `[tabId, editor]` registration effect re-run
 * (editor-pane.tsx's `useEffect(() => { ... registerEditorInstance(tabId, editor) ... },
 * [tabId, editor])`) — React tears down the previous commit's effect (unregistering the OLD
 * tabId) before running the new one (registering the NEW tabId with whatever `editor` value this
 * commit rendered). Uses the real `registerEditorInstance`/`unregisterEditorInstance` (not test
 * doubles), so this exercises the actual registry module the crash reaches through.
 */
const rerunRegistrationEffect = (previousTabId: string, nextTabId: string, editorForThisCommit: monaco.editor.IStandaloneCodeEditor | null) => {
    unregisterEditorInstance(previousTabId)
    if (editorForThisCommit) registerEditorInstance(nextTabId, editorForThisCommit)
}

/**
 * Locks down the registry-level contract the 2026-08-20 blank-window hotfix (contract §1-2)
 * depends on: a `canRenderCodeEditor`-false commit must never leave a dispose-bound instance
 * registered under the tab it just switched to. Calls the real `resolveEditorStateForRender` —
 * the same function `editor-pane.tsx` calls — rather than re-deriving its ternary, so a bug in
 * the shared decision function itself (not just its call site) fails this test. It does not
 * exercise `editor-pane.tsx`'s own wiring (that the render-phase `if` actually calls `setEditor`
 * with this function's result) — that wiring is guarded by review only, same as the rest of this
 * component's render-phase logic (contract §5): deleting both the `if` and its
 * `resolveEditorStateForRender` import leaves no unused symbol behind, so `tsc --noEmit` and
 * eslint both stay clean against that regression.
 */
describe('editor-pane.tsx 등록 effect 계약: canRenderCodeEditor 가 false 인 커밋에서 dispose 대상 인스턴스를 새 tabId 로 등록하지 않는다 (2026-08-20 빈 창 크래시)', () => {
    test('resolveEditorStateForRender 로 조정된 editor 상태를 등록 effect 에 넘기면, dispose 대상 인스턴스가 새 tabId 로 등록되지 않는다', () => {
        const { editor: staleLiveEditorFromOldTab, dispose: disposeStaleEditor } = createDisposableFakeEditor()
        registerEditorInstance('tab-old', staleLiveEditorFromOldTab)

        const isCacheMissForNewTab = true
        disposeStaleEditor()
        const editorAfterRenderPhaseAdjustment = resolveEditorStateForRender(staleLiveEditorFromOldTab, isCacheMissForNewTab, false, undefined)

        rerunRegistrationEffect('tab-old', 'tab-new', editorAfterRenderPhaseAdjustment)

        expect(getEditorInstance('tab-new')).toBeNull()
        expect(getEditorInstance('tab-old')).toBeNull()
    })
})

/**
 * Mirrors monaco's own `_actions` bookkeeping closely enough to reproduce the residual crash
 * path's mechanism, verified against the installed `monaco-editor` package source:
 * - `addAction` (`standaloneCodeEditor.js`'s `addAction`) has no disposed guard at all — it
 *   unconditionally `this._actions.set(id, new InternalEditorAction(..., this._contextKeyService))`,
 *   capturing whatever context-key service the editor currently holds, live or disposed.
 * - `dispose()` (`codeEditorWidget.js`'s `dispose`) clears `_actions` *before* disposing the
 *   context-key service via `super.dispose()` — so a disposed instance's action map starts empty,
 *   but nothing stops it being refilled afterward.
 * - `getSupportedActions()` (`codeEditorWidget.js`) is `getActions().filter(action =>
 *   action.isSupported())`, and `InternalEditorAction.isSupported()` (`editorAction.js`) calls
 *   `this._contextKeyService.contextMatchesRules(...)`, which throws `AbstractContextKeyService
 *   has been disposed` (`contextKeyService.js`'s `contextMatchesRules`) once the captured service
 *   is disposed.
 */
const createRechargeableFakeEditor = () => {
    const state = { contextDisposed: false, actions: new Map<string, { isSupported: () => boolean }>() }

    const addAction = (descriptor: { id: string }) => {
        state.actions.set(descriptor.id, {
            isSupported: () => {
                if (state.contextDisposed) throw new Error('AbstractContextKeyService has been disposed')
                return true
            },
        })
        return { dispose: () => state.actions.delete(descriptor.id) }
    }

    const dispose = () => {
        state.actions.clear()
        state.contextDisposed = true
    }

    const getSupportedActions = () => Array.from(state.actions.values()).filter((action) => action.isSupported())

    const editor = { addAction, dispose, getSupportedActions } as unknown as monaco.editor.IStandaloneCodeEditor
    return { editor, dispose, state }
}

/**
 * Models `useEditorGitGutterAndConflicts`'s two `addAction` effects (the "stage selection" and
 * "file history" context-menu actions) re-running in a commit's passive CREATE pass — their `path`
 * dependency changed alongside `tabId`, so React tears down and re-runs them in the same commit —
 * each guarded only by `if (!editor) return`, with no check for whether that `editor` has already
 * been disposed earlier in this same commit's destroy pass.
 */
const rerunGitGutterAddActionEffects = (editorForThisCommit: monaco.editor.IStandaloneCodeEditor | null) => {
    if (!editorForThisCommit) return
    editorForThisCommit.addAction({ id: 'taide.gitStageSelection', label: 'Stage Changes', run: () => undefined })
    editorForThisCommit.addAction({ id: OPEN_FILE_HISTORY_MONACO_ACTION_ID, label: 'File History', run: () => undefined })
}

/**
 * Reproduces the residual crash path the 2026-08-20 hotfix (`resolveEditorStateForRender`) does
 * not cover: a commit where `canRenderCodeEditor` stays true throughout (file already loaded, tier
 * fine) but `CodeEditor` still unmounts and remounts because a sibling JSX branch flips element
 * type — the markdown-preview split (`<Group>`) against a bare `<div>`, before `editor-pane.tsx`
 * was restructured to always render `<Group>` (contract §7). Since `resolveEditorStateForRender`
 * never fires in that commit (its `canRenderCodeEditor` input is true), the registration effect
 * re-registers the STALE `editor` state — already disposed by `CodeEditor`'s own passive
 * cleanup earlier in the same destroy pass — under the new tabId. This alone would be harmless
 * (dispose already cleared the corpse's action map, so `getSupportedActions()` returns `[]`), but
 * `useEditorGitGutterAndConflicts`'s two `addAction` effects — called earlier in `EditorPane`'s
 * body than the `[tabId, editor]` registration effect, so queued earlier in this commit's passive
 * effect list — refill that corpse's action map in this same commit's create pass, BEFORE the
 * registration effect creates and re-registers the same, now-recharged corpse under the new
 * tabId; their only guard, `!editor`, does not know the instance is disposed. From there the
 * throw surfaces one of two ways depending on whether `editor-area.tsx`'s globally-focused tab
 * also changed in this commit: if it didn't, `registerEditorInstance`'s synchronous
 * `notifyTabListeners` call (still subscribed to this tab from a prior commit) invokes
 * `editor-area.tsx`'s `attachToEditor` — and so `getSupportedActions()` — from inside this very
 * registration effect; if it did, `editor-area.tsx`'s own `[focusedFileTabId]` effect instead
 * tore its old subscription down in this commit's destroy pass and calls `attachToEditor()`
 * directly when its own create pass runs, throwing there instead (the case the original crash's
 * `<EditorArea>` stack attribution matches).
 *
 * This test cannot exercise `editor-pane.tsx`'s actual JSX (no RTL/monaco render harness in this
 * project — see contract §5) and so cannot fail against the pre-fix commit and pass against the
 * post-fix one; the structural fix (always rendering `<Group>`, closing the element-type flip
 * that produces this commit shape at all) is instead guarded by review only — deleting it leaves
 * no unused symbol, so neither `tsc --noEmit` nor eslint catch its regression. What this locks
 * down is the *mechanism*, against a hand-written mock rather than monaco itself: that recreating
 * this commit shape reliably reproduces the throw. It does not evaluate the registration-effect
 * disposed-guard contract §2 rejects — that guard would in fact have kept this corpse out of the
 * registry too, closing this symptom just as well; contract §2's actual reason for rejecting it
 * is that it only launders the registry's own bookkeeping and leaves the underlying invariant
 * (registered instance ≡ live instance) unenforced for any consumer that reads `EditorPane`'s
 * `editor` state directly instead of going through the registry (e.g.
 * `use-editor-file-persistence.ts`'s `editor?.getAction(FORMAT_DOCUMENT_ACTION_ID)`).
 */
describe('잔존 경로: 프리뷰 Group↔div 플립 커밋에서 dispose 된 corpse 가 git-gutter addAction 으로 재충전되어 throw 한다', () => {
    test('dispose 후 addAction 재충전 없이 getSupportedActions 를 호출하면 throw 하지 않는다 (dispose 만으로는 무해)', () => {
        const { editor: corpse, dispose } = createRechargeableFakeEditor()
        registerEditorInstance('tab-old-quiet', corpse)

        dispose()
        rerunRegistrationEffect('tab-old-quiet', 'tab-new-quiet', corpse)

        expect(() => getEditorInstance('tab-new-quiet')?.getSupportedActions()).not.toThrow()
        expect(getEditorInstance('tab-new-quiet')?.getSupportedActions()).toEqual([])

        unregisterEditorInstance('tab-new-quiet')
    })

    test('dispose 후 같은 커밋에서 git-gutter addAction 이 재충전하면 getSupportedActions 가 throw 한다', () => {
        const { editor: corpse, dispose } = createRechargeableFakeEditor()
        registerEditorInstance('tab-old', corpse)

        dispose()
        rerunGitGutterAddActionEffects(corpse)
        rerunRegistrationEffect('tab-old', 'tab-new', corpse)

        const registeredCorpse = getEditorInstance('tab-new')
        expect(registeredCorpse).not.toBeNull()
        expect(() => registeredCorpse?.getSupportedActions()).toThrow('AbstractContextKeyService has been disposed')

        unregisterEditorInstance('tab-new')
    })
})
