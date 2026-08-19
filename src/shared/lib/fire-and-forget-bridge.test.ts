import { describe, expect, test } from 'bun:test'
import { createFireAndForgetBridge } from '@shared/lib/fire-and-forget-bridge'

describe('createFireAndForgetBridge / broadcast + ignore(기본값)', () => {
    test('구독 중 발행하면 모든 리스너가 페이로드를 받는다', () => {
        const bridge = createFireAndForgetBridge<string>()
        let first: string | undefined
        let second: string | undefined
        const unsubscribeFirst = bridge.subscribe((payload) => (first = payload))
        const unsubscribeSecond = bridge.subscribe((payload) => (second = payload))

        bridge.publish('hello')
        unsubscribeFirst()
        unsubscribeSecond()

        expect(first).toBe('hello')
        expect(second).toBe('hello')
    })

    test('구독 해제 후에는 호출되지 않는다', () => {
        const bridge = createFireAndForgetBridge<string>()
        let calls = 0
        const unsubscribe = bridge.subscribe(() => (calls += 1))
        unsubscribe()

        bridge.publish('hello')

        expect(calls).toBe(0)
    })

    test('구독자가 없을 때 발행하면 조용히 버려진다', () => {
        const bridge = createFireAndForgetBridge<string>()
        bridge.publish('lost')

        let received: string | undefined
        const unsubscribe = bridge.subscribe((payload) => (received = payload))
        unsubscribe()

        expect(received).toBeUndefined()
    })

    test('hasSubscribers 는 구독자 존재 여부를 반영한다', () => {
        const bridge = createFireAndForgetBridge<string>()
        expect(bridge.hasSubscribers()).toBe(false)

        const unsubscribe = bridge.subscribe(() => {})
        expect(bridge.hasSubscribers()).toBe(true)

        unsubscribe()
        expect(bridge.hasSubscribers()).toBe(false)
    })
})

describe('createFireAndForgetBridge / broadcast + queue-latest', () => {
    test('구독자가 없을 때의 발행은 보류되었다가 다음 구독 시 전달된다', () => {
        const bridge = createFireAndForgetBridge<string>({ emptyPolicy: 'queue-latest' })
        bridge.publish('first')

        let received: string | undefined
        const unsubscribe = bridge.subscribe((payload) => (received = payload))
        unsubscribe()

        expect(received).toBe('first')
    })

    test('구독자 없이 여러 번 발행하면 가장 최근 값만 유지된다', () => {
        const bridge = createFireAndForgetBridge<string>({ emptyPolicy: 'queue-latest' })
        bridge.publish('first')
        bridge.publish('second')

        let received: string | undefined
        const unsubscribe = bridge.subscribe((payload) => (received = payload))
        unsubscribe()

        expect(received).toBe('second')
    })

    test('보류된 값은 최초 구독자에게 한 번만 전달된다', () => {
        const bridge = createFireAndForgetBridge<string>({ emptyPolicy: 'queue-latest' })
        bridge.publish('first')

        let firstCalls = 0
        const unsubscribeFirst = bridge.subscribe(() => (firstCalls += 1))
        unsubscribeFirst()

        let secondCalls = 0
        const unsubscribeSecond = bridge.subscribe(() => (secondCalls += 1))
        unsubscribeSecond()

        expect(firstCalls).toBe(1)
        expect(secondCalls).toBe(0)
    })
})

describe('createFireAndForgetBridge / broadcast + queue-all', () => {
    test('구독자가 없을 때의 발행들은 순서대로 쌓였다가 구독 시 한꺼번에 전달된다', () => {
        const bridge = createFireAndForgetBridge<string>({ emptyPolicy: 'queue-all' })
        bridge.publish('first')
        bridge.publish('second')

        const received: string[] = []
        const unsubscribe = bridge.subscribe((payload) => received.push(payload))
        unsubscribe()

        expect(received).toEqual(['first', 'second'])
    })
})

describe('createFireAndForgetBridge / single-owner', () => {
    test('나중에 구독한 리스너가 이전 리스너를 대체한다', () => {
        const bridge = createFireAndForgetBridge<string>({ subscriberModel: 'single-owner' })
        const firstReceived: string[] = []
        const secondReceived: string[] = []
        bridge.subscribe((payload) => firstReceived.push(payload))
        bridge.subscribe((payload) => secondReceived.push(payload))

        bridge.publish('hello')

        expect(firstReceived).toEqual([])
        expect(secondReceived).toEqual(['hello'])
    })

    test('교체된 이전 리스너의 구독 해제는 새 리스너에 영향을 주지 않는다', () => {
        const bridge = createFireAndForgetBridge<string>({ subscriberModel: 'single-owner' })
        const unsubscribeFirst = bridge.subscribe(() => {})
        const secondReceived: string[] = []
        bridge.subscribe((payload) => secondReceived.push(payload))

        unsubscribeFirst()
        bridge.publish('hello')

        expect(secondReceived).toEqual(['hello'])
    })

    test('single-owner + queue-all 은 소유자 등록 시 밀린 발행을 순서대로 전달한다', () => {
        const bridge = createFireAndForgetBridge<string>({ subscriberModel: 'single-owner', emptyPolicy: 'queue-all' })
        bridge.publish('first')
        bridge.publish('second')

        const received: string[] = []
        bridge.subscribe((payload) => received.push(payload))

        expect(received).toEqual(['first', 'second'])
    })
})
