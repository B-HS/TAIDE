import { describe, expect, test } from 'bun:test'
import type { KeyboardEvent } from 'react'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'

const makeEvent = (key: string, { target, currentTarget, repeat }: { target?: unknown; currentTarget?: unknown; repeat?: boolean } = {}) => {
    const self = {}
    const event = {
        key,
        repeat: repeat ?? false,
        target: target ?? self,
        currentTarget: currentTarget ?? self,
        defaultPrevented: false,
        preventDefault: () => {
            event.defaultPrevented = true
        },
    }
    return event as unknown as KeyboardEvent
}

describe('createActivationKeyDownHandler', () => {
    test('Enter 키에서 콜백을 호출한다', () => {
        let called = false
        const handler = createActivationKeyDownHandler(() => {
            called = true
        })
        handler(makeEvent('Enter'))
        expect(called).toBe(true)
    })

    test('Space 키에서 콜백을 호출하고 기본 스크롤 동작을 막는다', () => {
        let called = false
        const handler = createActivationKeyDownHandler(() => {
            called = true
        })
        const event = makeEvent(' ')
        handler(event)
        expect(called).toBe(true)
        expect(event.defaultPrevented).toBe(true)
    })

    test('그 외 키에서는 콜백을 호출하지 않는다', () => {
        let called = false
        const handler = createActivationKeyDownHandler(() => {
            called = true
        })
        handler(makeEvent('Tab'))
        expect(called).toBe(false)
    })

    test('Space 키를 누르고 있어도 auto-repeat 중에는 콜백을 다시 호출하지 않되 스크롤은 계속 막는다', () => {
        let callCount = 0
        const handler = createActivationKeyDownHandler(() => {
            callCount += 1
        })
        handler(makeEvent(' '))
        const repeatedEvent = makeEvent(' ', { repeat: true })
        handler(repeatedEvent)
        handler(repeatedEvent)
        expect(callCount).toBe(1)
        expect(repeatedEvent.defaultPrevented).toBe(true)
    })

    test('중첩된 자식 요소에서 버블링된 키 이벤트는 무시한다', () => {
        let called = false
        const handler = createActivationKeyDownHandler(() => {
            called = true
        })
        const row = {}
        const nestedChildButton = {}
        handler(makeEvent('Enter', { target: nestedChildButton, currentTarget: row }))
        expect(called).toBe(false)
    })
})
