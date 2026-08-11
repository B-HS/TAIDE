import { describe, expect, test } from 'bun:test'
import { requestEditorPaneCommand, subscribeEditorPaneCommand } from '@shared/lib/editor-pane-command-bridge'

describe('editorPaneCommandBridge', () => {
    test('구독한 리스너는 요청한 커맨드를 그대로 전달받는다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'cycle-tab', direction: 'next' })
        unsubscribe()

        expect(received).toEqual({ type: 'cycle-tab', direction: 'next' })
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeEditorPaneCommand(() => {
            calls += 1
        })
        unsubscribe()

        requestEditorPaneCommand({ type: 'split' })

        expect(calls).toBe(0)
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first = 0
        let second = 0
        const unsubscribeFirst = subscribeEditorPaneCommand(() => {
            first += 1
        })
        const unsubscribeSecond = subscribeEditorPaneCommand(() => {
            second += 1
        })

        requestEditorPaneCommand({ type: 'save-active-tab' })
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe(1)
        expect(second).toBe(1)
    })
})
