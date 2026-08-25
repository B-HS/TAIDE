import { describe, expect, test } from 'bun:test'
import {
    getActiveEditorActionIdsSnapshot,
    setActiveEditorActionIds,
    subscribeActiveEditorActionIds,
} from '@shared/lib/bridge/active-editor-actions-bridge'

describe('activeEditorActionsBridge', () => {
    test('초기 스냅샷은 null 이다', () => {
        expect(getActiveEditorActionIdsSnapshot()).toBeNull()
    })

    test('setActiveEditorActionIds 는 스냅샷을 갱신한다', () => {
        const ids = new Set(['editor.action.formatDocument'])
        setActiveEditorActionIds(ids)

        expect(getActiveEditorActionIdsSnapshot()).toBe(ids)

        setActiveEditorActionIds(null)
    })

    test('구독 중인 모든 리스너가 변경 시 호출된다', () => {
        let firstCalls = 0
        let secondCalls = 0
        const unsubscribeFirst = subscribeActiveEditorActionIds(() => (firstCalls += 1))
        const unsubscribeSecond = subscribeActiveEditorActionIds(() => (secondCalls += 1))

        setActiveEditorActionIds(new Set(['a']))
        unsubscribeFirst()
        unsubscribeSecond()
        setActiveEditorActionIds(null)

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(1)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeActiveEditorActionIds(() => (calls += 1))
        unsubscribe()

        setActiveEditorActionIds(new Set(['a']))
        setActiveEditorActionIds(null)

        expect(calls).toBe(0)
    })
})
