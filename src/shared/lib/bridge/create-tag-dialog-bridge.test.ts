import { describe, expect, test } from 'bun:test'
import { requestOpenCreateTagDialog, subscribeOpenCreateTagDialog } from '@shared/lib/bridge/create-tag-dialog-bridge'

describe('createTagDialogBridge / 구독 중 요청', () => {
    test('구독된 상태에서 요청하면 즉시 리스너가 호출된다', () => {
        let received: unknown = null
        const unsubscribe = subscribeOpenCreateTagDialog((request) => {
            received = request
        })

        requestOpenCreateTagDialog({ target: 'HEAD' })
        unsubscribe()

        expect(received).toEqual({ target: 'HEAD' })
    })

    test('여러 리스너가 구독 중이면 모두 호출된다', () => {
        let first: unknown = null
        let second: unknown = null
        const unsubscribeFirst = subscribeOpenCreateTagDialog((request) => {
            first = request
        })
        const unsubscribeSecond = subscribeOpenCreateTagDialog((request) => {
            second = request
        })

        requestOpenCreateTagDialog({ target: 'HEAD' })
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toEqual({ target: 'HEAD' })
        expect(second).toEqual({ target: 'HEAD' })
    })
})

describe('createTagDialogBridge / 구독자 없이 요청(보류)', () => {
    test('구독자가 없을 때의 요청은 보류되었다가 다음 구독 시 즉시 전달된다', () => {
        requestOpenCreateTagDialog({ target: 'abc123' })

        let received: unknown = null
        const unsubscribe = subscribeOpenCreateTagDialog((request) => {
            received = request
        })
        unsubscribe()

        expect(received).toEqual({ target: 'abc123' })
    })

    test('보류된 요청은 최초 구독자에게 한 번만 전달된다', () => {
        requestOpenCreateTagDialog({ target: 'first' })

        let firstCalls = 0
        const unsubscribeFirst = subscribeOpenCreateTagDialog(() => {
            firstCalls += 1
        })
        unsubscribeFirst()

        let secondCalls = 0
        const unsubscribeSecond = subscribeOpenCreateTagDialog(() => {
            secondCalls += 1
        })
        unsubscribeSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(0)
    })
})

describe('createTagDialogBridge / 구독 해제', () => {
    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenCreateTagDialog(() => {
            calls += 1
        })
        unsubscribe()

        requestOpenCreateTagDialog({ target: 'HEAD' })

        expect(calls).toBe(0)
    })
})
