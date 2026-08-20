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
 * adjustment and `CodeEditor`'s own registration effect route around. The tests below only check
 * which `tabId` ends up holding a reference in `editor-instance-registry` — they never call a
 * method on the value itself — so unlike {@link createRechargeableFakeEditor} below, this does not
 * need to model monaco's action-map bookkeeping (an earlier revision gave it a
 * `getSupportedActions` that threw immediately on dispose, contradicting
 * `createRechargeableFakeEditor`'s dispose-alone-is-harmless model below without ever actually
 * being exercised by an assertion; removed).
 */
const createDisposableFakeEditor = () => {
    const editor = {} as unknown as monaco.editor.IStandaloneCodeEditor
    return { editor, dispose: () => undefined }
}

/**
 * Simulates `CodeEditor`'s own `[registryTabId]` effect (code-editor.tsx, contract
 * crash-class-seal-contract.md §1-1) re-keying the SAME live instance to a new `tabId` while the
 * component stays mounted — the effect's cleanup unregisters the OLD `registryTabId` before its
 * setup registers under the NEW one, exactly as a single hook's own cleanup-then-setup is always
 * sequenced by React on a dependency change. `editor` here is always the caller's own live
 * instance — unlike the pre-move `EditorPane`-owned `[tabId, editor]` effect this replaces, there
 * is no parent `editor` state snapshot that could still reference a foreign, already-disposed
 * instance. Uses the real `registerEditorInstance`/`unregisterEditorInstance` (not test doubles),
 * so this exercises the actual registry module.
 */
const rerunChildOwnedRegistrationEffect = (previousTabId: string, nextTabId: string, editor: monaco.editor.IStandaloneCodeEditor) => {
    unregisterEditorInstance(previousTabId)
    registerEditorInstance(nextTabId, editor)
}

/**
 * Simulates the same effect's cleanup running alone with no following setup — the shape a full
 * `CodeEditor` unmount takes. There is no "new tabId" to register under because the component
 * that owned this effect is gone; on full unmount `registryTabId`'s effect only tears itself down
 * (contract crash-class-seal-contract.md §1-1's "on full unmount this effect only runs its
 * cleanup").
 */
const unmountChildOwnedRegistrationEffect = (tabId: string) => unregisterEditorInstance(tabId)

/**
 * Locks down the registry-level contract crash-class-seal-contract.md §1-1 moved onto
 * `CodeEditor` itself: because registration is now keyed off `CodeEditor`'s own
 * `editorRef.current` rather than a parent's `editor` state snapshot, there is no code path left
 * that can register a disposed instance under a new `tabId` at all — re-keying only ever moves a
 * still-live instance, and a full unmount only ever unregisters. This supersedes the pre-move
 * version of this contract, which instead had to prove that `resolveEditorStateForRender`'s
 * render-phase adjustment kept a since-removed `EditorPane`-owned registration effect from
 * re-registering a corpse; that effect and its dependency on `resolveEditorStateForRender` no
 * longer exist (`resolveEditorStateForRender` still runs in `editor-pane.tsx`, but only to protect
 * the OTHER hooks — `useEditorGitGutterAndConflicts`, `useEditorBlame`,
 * `useEditorFilePersistence`, `useEditorViewState`, `useEditorIdeSelection` — that still read
 * `editor` state directly instead of through this registry).
 */
describe('CodeEditor 등록 effect 계약(자식 소유 모델, crash-class-seal-contract.md §1-1): registry 항목은 항상 CodeEditor 자신의 live 인스턴스이거나 비어 있다', () => {
    test('tabId 재키잉은 같은 live 인스턴스를 새 tabId 로 옮길 뿐, dispose 된 인스턴스를 등록하지 않는다', () => {
        const { editor: liveEditor } = createDisposableFakeEditor()
        registerEditorInstance('tab-old', liveEditor)

        rerunChildOwnedRegistrationEffect('tab-old', 'tab-new', liveEditor)

        expect(getEditorInstance('tab-old')).toBeNull()
        expect(getEditorInstance('tab-new')).toBe(liveEditor)

        unregisterEditorInstance('tab-new')
    })

    test('CodeEditor 전체 언마운트 시퀀스(dispose 후 등록 effect cleanup)에서 registry 가 dispose 된 인스턴스를 갖는 순간이 없다', () => {
        const { editor: liveEditor, dispose } = createDisposableFakeEditor()
        registerEditorInstance('tab-a', liveEditor)

        dispose()
        unmountChildOwnedRegistrationEffect('tab-a')

        expect(getEditorInstance('tab-a')).toBeNull()
    })

    test('tabId 전환 이후 언마운트까지 이어지는 시퀀스에서도 registry 가 dispose 된 인스턴스를 갖는 순간이 없다', () => {
        const { editor: liveEditor, dispose } = createDisposableFakeEditor()
        registerEditorInstance('tab-a', liveEditor)

        rerunChildOwnedRegistrationEffect('tab-a', 'tab-b', liveEditor)
        expect(getEditorInstance('tab-a')).toBeNull()
        expect(getEditorInstance('tab-b')).toBe(liveEditor)

        dispose()
        unmountChildOwnedRegistrationEffect('tab-b')

        expect(getEditorInstance('tab-a')).toBeNull()
        expect(getEditorInstance('tab-b')).toBeNull()
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
 * Documents a crash path that predates the 2026-08-20 hotfix restructure and is now closed by TWO
 * independent later changes, neither of which this file's mechanism-level mock can exercise
 * directly (see the "cannot exercise" paragraph below) — kept as mechanism documentation, not as a
 * still-reachable regression test:
 *
 * 1. blank-window-hotfix-contract.md §7.1 (`editor-pane.tsx`): the commit shape this path needs —
 *    `canRenderCodeEditor` staying true throughout while `CodeEditor` still unmounts and remounts
 *    because a sibling JSX branch flips element type (the markdown-preview split, `<Group>` vs a
 *    bare `<div>`) — no longer occurs at all; `editor-pane.tsx` now always renders the same
 *    `<Group>`+editor `<Panel>` regardless of preview state.
 * 2. crash-class-seal-contract.md §1-1 (`code-editor.tsx`): even if that commit shape recurred via
 *    a future regression, registration no longer has a code path that could re-register a stale
 *    instance under a new `tabId` — `EditorPane`'s old `[tabId, editor]` registration effect (the
 *    one this mock's `rerunChildOwnedRegistrationEffect` originally stood in for) no longer
 *    exists; registration is now `CodeEditor`'s own effect, keyed off its own `editorRef.current`,
 *    which can only ever be that same instance's live editor or nothing.
 *
 * What these two tests still document accurately is monaco's OWN dispose/recharge mechanism
 * (verified against the installed `monaco-editor` source, JSDoc above `createRechargeableFakeEditor`):
 * `addAction` has no disposed guard, `dispose()` clears `_actions` before disposing the
 * context-key service, and an `addAction` call after dispose silently refills the action map with
 * an entry that captures the disposed service — so a later `getSupportedActions()` throws. That
 * mechanism is still real and still matters: `useEditorGitGutterAndConflicts`'s two `addAction`
 * effects (modeled by {@link rerunGitGutterAddActionEffects}) read `EditorPane`'s `editor` state
 * DIRECTLY, not through the registry, so they are the reason `resolveEditorStateForRender`'s
 * render-phase adjustment in `editor-pane.tsx` still exists and is still required — it is what
 * keeps `editor` state itself from ever holding a disposed instance for git-gutter (or blame, or
 * file-persistence) to recharge in the first place, now that the registry can no longer be the
 * thing that turns a recharge into a thrown, root-unmounting error.
 *
 * These tests cannot exercise `editor-pane.tsx`'s actual JSX (no RTL/monaco render harness in this
 * project — see contract §5) and so cannot fail against a pre-fix commit and pass against a
 * post-fix one; both structural fixes above are guarded by review only.
 */
describe('잔존 경로(역사적 기록, 지금은 §7.1 + crash-class-seal-contract §1-1 로 이중 봉인): dispose 된 corpse 가 git-gutter addAction 으로 재충전되면 getSupportedActions 가 throw 하는 monaco 자체의 메커니즘', () => {
    test('dispose 후 addAction 재충전 없이 getSupportedActions 를 호출하면 throw 하지 않는다 (dispose 만으로는 무해)', () => {
        const { editor: corpse, dispose } = createRechargeableFakeEditor()
        registerEditorInstance('tab-old-quiet', corpse)

        dispose()
        rerunChildOwnedRegistrationEffect('tab-old-quiet', 'tab-new-quiet', corpse)

        expect(() => getEditorInstance('tab-new-quiet')?.getSupportedActions()).not.toThrow()
        expect(getEditorInstance('tab-new-quiet')?.getSupportedActions()).toEqual([])

        unregisterEditorInstance('tab-new-quiet')
    })

    test('dispose 후 같은 커밋에서 git-gutter addAction 이 재충전하면 getSupportedActions 가 throw 한다', () => {
        const { editor: corpse, dispose } = createRechargeableFakeEditor()
        registerEditorInstance('tab-old', corpse)

        dispose()
        rerunGitGutterAddActionEffects(corpse)
        rerunChildOwnedRegistrationEffect('tab-old', 'tab-new', corpse)

        const registeredCorpse = getEditorInstance('tab-new')
        expect(registeredCorpse).not.toBeNull()
        expect(() => registeredCorpse?.getSupportedActions()).toThrow('AbstractContextKeyService has been disposed')

        unregisterEditorInstance('tab-new')
    })
})
