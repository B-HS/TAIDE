import { describe, expect, test } from 'bun:test'
import { createExternalStoreBridge } from '@shared/lib/bridge/external-store-bridge'

describe('createExternalStoreBridge', () => {
    test('getSnapshot 은 초기값을 반환한다', () => {
        const store = createExternalStoreBridge<number | null>(null)
        expect(store.getSnapshot()).toBeNull()
    })

    test('setValue 는 getSnapshot 이 반환하는 값을 갱신한다', () => {
        const store = createExternalStoreBridge<number | null>(null)
        store.setValue(3)
        expect(store.getSnapshot()).toBe(3)
    })

    test('setValue 는 구독 중인 모든 리스너에게 변경을 알린다', () => {
        const store = createExternalStoreBridge<number | null>(null)
        let firstCalls = 0
        let secondCalls = 0
        const unsubscribeFirst = store.subscribe(() => (firstCalls += 1))
        const unsubscribeSecond = store.subscribe(() => (secondCalls += 1))

        store.setValue(1)
        unsubscribeFirst()
        unsubscribeSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(1)
    })

    test('구독 해제 후에는 알림을 받지 않는다', () => {
        const store = createExternalStoreBridge<number | null>(null)
        let calls = 0
        const unsubscribe = store.subscribe(() => (calls += 1))
        unsubscribe()

        store.setValue(1)

        expect(calls).toBe(0)
    })
})
