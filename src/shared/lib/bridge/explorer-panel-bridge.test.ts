import { describe, expect, test } from 'bun:test'
import {
    requestShowExplorerView,
    requestToggleExplorerSidebar,
    subscribeShowExplorerView,
    subscribeToggleExplorerSidebar,
} from '@shared/lib/bridge/explorer-panel-bridge'

describe('explorerPanelBridge / toggleExplorerSidebar', () => {
    test('구독한 리스너는 요청 시 호출된다', () => {
        let calls = 0
        const unsubscribe = subscribeToggleExplorerSidebar(() => {
            calls += 1
        })

        requestToggleExplorerSidebar()
        requestToggleExplorerSidebar()
        unsubscribe()

        expect(calls).toBe(2)
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeToggleExplorerSidebar(() => {
            calls += 1
        })
        unsubscribe()

        requestToggleExplorerSidebar()

        expect(calls).toBe(0)
    })
})

describe('explorerPanelBridge / showExplorerView', () => {
    test('요청한 view 값이 그대로 리스너에 전달된다', () => {
        let received: unknown
        const unsubscribe = subscribeShowExplorerView((view) => {
            received = view
        })

        requestShowExplorerView('git')
        unsubscribe()

        expect(received).toBe('git')
    })

    test('여러 리스너가 모두 호출된다', () => {
        let first: unknown
        let second: unknown
        const unsubscribeFirst = subscribeShowExplorerView((view) => {
            first = view
        })
        const unsubscribeSecond = subscribeShowExplorerView((view) => {
            second = view
        })

        requestShowExplorerView('files')
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe('files')
        expect(second).toBe('files')
    })
})
