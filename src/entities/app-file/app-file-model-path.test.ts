import { describe, expect, test } from 'bun:test'
import type { AppFileTarget } from '@shared/api/bindings'
import { resolveAppFileModelPath } from '@entities/app-file/app-file-model-path'

describe('resolveAppFileModelPath', () => {
    test('settings 타깃은 고정된 settings.json 경로를 반환한다', () => {
        expect(resolveAppFileModelPath({ kind: 'settings' })).toBe('/__app-file__/settings.json')
    })

    test('prompt 타깃은 id 별로 서로 다른 경로를 반환한다', () => {
        const autoTab: AppFileTarget = { kind: 'prompt', id: 'auto-tab-default' }
        const inlineEdit: AppFileTarget = { kind: 'prompt', id: 'inline-edit-default' }

        expect(resolveAppFileModelPath(autoTab)).toBe('/__app-file__/prompt/auto-tab-default.json')
        expect(resolveAppFileModelPath(inlineEdit)).toBe('/__app-file__/prompt/inline-edit-default.json')
        expect(resolveAppFileModelPath(autoTab)).not.toBe(resolveAppFileModelPath(inlineEdit))
    })
})
