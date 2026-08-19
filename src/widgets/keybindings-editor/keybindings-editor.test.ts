import { describe, expect, test } from 'bun:test'
import { KeybindingsEditor } from '@widgets/keybindings-editor/keybindings-editor'

/**
 * No `@testing-library/react`/DOM environment is configured for `bun:test` in this project, so this
 * stays a load-only smoke test — `open`/`onOpenChange` are now controlled props owned by
 * `KeybindingsRuntimeProvider` (see that provider's own doc comment and test), which is what moved
 * the "open on bridge request" and "apply monaco overrides" effects out of this component.
 */
describe('KeybindingsEditor 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', () => {
        expect(typeof KeybindingsEditor).toBe('function')
    })
})
