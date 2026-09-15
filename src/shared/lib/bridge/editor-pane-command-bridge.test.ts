import { describe, expect, test } from 'bun:test'
import { requestEditorPaneCommand, subscribeEditorPaneCommand } from '@shared/lib/bridge/editor-pane-command-bridge'

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

    test('run-selected-text-in-terminal 커맨드는 페이로드 없이 그대로 전달된다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'run-selected-text-in-terminal' })
        unsubscribe()

        expect(received).toEqual({ type: 'run-selected-text-in-terminal' })
    })

    test('focus-group 커맨드는 방향 타깃을 그대로 전달한다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'direction', direction: 'left' } })
        unsubscribe()

        expect(received).toEqual({ type: 'focus-group', target: { kind: 'direction', direction: 'left' } })
    })

    test('focus-group 커맨드는 위치 타깃(⌘1~⌘9)도 그대로 전달한다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'focus-group', target: { kind: 'position', position: 3 } })
        unsubscribe()

        expect(received).toEqual({ type: 'focus-group', target: { kind: 'position', position: 3 } })
    })

    test('move-tab-to-group 커맨드는 방향을 그대로 전달한다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'move-tab-to-group', direction: 'right' })
        unsubscribe()

        expect(received).toEqual({ type: 'move-tab-to-group', direction: 'right' })
    })

    test('close-all-tabs 커맨드는 페이로드 없이 그대로 전달된다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'close-all-tabs' })
        unsubscribe()

        expect(received).toEqual({ type: 'close-all-tabs' })
    })

    test('run-in-terminal 커맨드는 text·cwd 페이로드를 그대로 전달한다', () => {
        let received: unknown
        const unsubscribe = subscribeEditorPaneCommand((command) => {
            received = command
        })

        requestEditorPaneCommand({ type: 'run-in-terminal', text: 'npm run build', cwd: '/repo' })
        unsubscribe()

        expect(received).toEqual({ type: 'run-in-terminal', text: 'npm run build', cwd: '/repo' })
    })
})
