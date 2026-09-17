import { describe, expect, test } from 'bun:test'
import {
    clearExternallyDirtyMark,
    consumeExternallyDirtyModel,
    markModelDirtyExternally,
    onModelEditedExternally,
} from '@shared/lib/lsp/model-dirty-tracker'

describe('clearExternallyDirtyMark', () => {
    test('표시를 지워 다음 consume 이 false 를 보게 한다', () => {
        markModelDirtyExternally('/repo/cleared.ts')

        clearExternallyDirtyMark('/repo/cleared.ts')

        expect(consumeExternallyDirtyModel('/repo/cleared.ts')).toBe(false)
    })

    /**
     * The tab bar's dot listens on the same event `markModelDirtyExternally` publishes. A release is
     * not an edit — notifying here would raise a dirty dot on a tab that is being torn down, or on a
     * freshly reopened one.
     */
    test('마크를 지우는 것은 편집이 아니므로 구독자에게 알리지 않는다', () => {
        const notified: string[] = []
        const unsubscribe = onModelEditedExternally((path) => notified.push(path))

        clearExternallyDirtyMark('/repo/silent.ts')
        unsubscribe()

        expect(notified).toEqual([])
    })

    test('표시된 적 없는 경로를 지워도 아무 일도 일어나지 않는다', () => {
        clearExternallyDirtyMark('/repo/never-marked.ts')

        expect(consumeExternallyDirtyModel('/repo/never-marked.ts')).toBe(false)
    })
})
