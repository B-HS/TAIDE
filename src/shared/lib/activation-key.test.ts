import { describe, expect, test } from 'bun:test'
import type { KeyboardEvent } from 'react'
import { createActivationKeyDownHandler } from '@shared/lib/activation-key'

const makeEvent = (key: string) => {
    const event = {
        key,
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
})
