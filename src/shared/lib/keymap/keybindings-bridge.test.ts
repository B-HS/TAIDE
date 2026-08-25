import { describe, expect, test } from 'bun:test'
import { requestOpenKeybindingsEditor, subscribeOpenKeybindingsEditor } from '@shared/lib/keymap/keybindings-bridge'

describe('keybindingsBridge / openKeybindingsEditor', () => {
    test('구독한 리스너는 요청 시 호출된다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenKeybindingsEditor(() => {
            calls += 1
        })

        requestOpenKeybindingsEditor()
        requestOpenKeybindingsEditor()
        unsubscribe()

        expect(calls).toBe(2)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeOpenKeybindingsEditor(() => {
            calls += 1
        })
        unsubscribe()

        requestOpenKeybindingsEditor()

        expect(calls).toBe(0)
    })

    test('여러 리스너가 모두 호출된다', () => {
        let firstCalls = 0
        let secondCalls = 0
        const unsubscribeFirst = subscribeOpenKeybindingsEditor(() => {
            firstCalls += 1
        })
        const unsubscribeSecond = subscribeOpenKeybindingsEditor(() => {
            secondCalls += 1
        })

        requestOpenKeybindingsEditor()
        unsubscribeFirst()
        unsubscribeSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(1)
    })
})
