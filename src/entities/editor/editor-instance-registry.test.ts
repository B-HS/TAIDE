import { describe, expect, test } from 'bun:test'
import type { monaco } from '@shared/lib/monaco/setup'
import {
    getEditorInstance,
    registerEditorInstance,
    subscribeEditorInstance,
    unregisterEditorInstance,
} from '@entities/editor/editor-instance-registry'

const createFakeEditor = () => ({}) as unknown as monaco.editor.IStandaloneCodeEditor

describe('editor-instance-registry', () => {
    test('등록되지 않은 tabId 는 null 을 반환한다', () => {
        expect(getEditorInstance('tab-unregistered')).toBeNull()
    })

    test('등록한 editor 를 같은 tabId 로 조회할 수 있다', () => {
        const editor = createFakeEditor()
        registerEditorInstance('tab-a', editor)

        expect(getEditorInstance('tab-a')).toBe(editor)

        unregisterEditorInstance('tab-a')
    })

    test('등록 해제 후에는 null 을 반환한다', () => {
        registerEditorInstance('tab-b', createFakeEditor())
        unregisterEditorInstance('tab-b')

        expect(getEditorInstance('tab-b')).toBeNull()
    })

    test('같은 tabId 로 register/unregister 하면 그 tabId 의 리스너만 통지된다', () => {
        const notifiedTabIds: string[] = []
        const unsubscribeA = subscribeEditorInstance('tab-c', () => notifiedTabIds.push('tab-c'))
        const unsubscribeD = subscribeEditorInstance('tab-d', () => notifiedTabIds.push('tab-d'))

        registerEditorInstance('tab-c', createFakeEditor())
        expect(notifiedTabIds).toEqual(['tab-c'])

        unregisterEditorInstance('tab-c')
        expect(notifiedTabIds).toEqual(['tab-c', 'tab-c'])
        expect(notifiedTabIds).not.toContain('tab-d')

        unsubscribeA()
        unsubscribeD()
    })

    test('구독 해제 후에는 더 이상 통지되지 않는다', () => {
        let calls = 0
        const unsubscribe = subscribeEditorInstance('tab-e', () => {
            calls += 1
        })
        unsubscribe()

        registerEditorInstance('tab-e', createFakeEditor())
        unregisterEditorInstance('tab-e')

        expect(calls).toBe(0)
    })

    test('같은 editor 를 다른 tabId 로 재등록하면 이전 tabId 는 비워지고 새 tabId 로 조회된다', () => {
        const editor = createFakeEditor()
        registerEditorInstance('tab-old', editor)

        unregisterEditorInstance('tab-old')
        registerEditorInstance('tab-new', editor)

        expect(getEditorInstance('tab-old')).toBeNull()
        expect(getEditorInstance('tab-new')).toBe(editor)

        unregisterEditorInstance('tab-new')
    })
})
