import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import { getKeymapCapturingSnapshot, setKeymapCapturing, subscribeKeymapCapturing } from '@shared/lib/keymap/keymap-capture'

/**
 * A module-scope flag with a `useSyncExternalStore` shape: the keybindings editor raises it while
 * recording a chord so the global dispatcher stops swallowing keystrokes. Its state outlives any
 * one test (bun shares modules across files), so every test resets it and drops its subscriptions
 * — a leaked listener would keep firing into an unmounted test's arrays for the rest of the run.
 */
const releases: (() => void)[] = []

const subscribe = (listener: () => void) => {
    const release = subscribeKeymapCapturing(listener)
    releases.push(release)
    return release
}

beforeEach(() => setKeymapCapturing(false))

afterEach(() => {
    for (const release of releases.splice(0)) release()
    setKeymapCapturing(false)
})

describe('keymap-capture', () => {
    test('기본 스냅샷은 false 다', () => {
        expect(getKeymapCapturingSnapshot()).toBe(false)
    })

    test('값이 바뀌면 스냅샷이 갱신되고 구독자가 1회 호출된다', () => {
        let notifications = 0
        subscribe(() => {
            notifications += 1
        })

        setKeymapCapturing(true)

        expect(getKeymapCapturingSnapshot()).toBe(true)
        expect(notifications).toBe(1)
    })

    test('같은 값을 다시 설정하면 알리지 않는다 (useSyncExternalStore 무한 렌더 방지)', () => {
        setKeymapCapturing(true)
        let notifications = 0
        subscribe(() => {
            notifications += 1
        })

        setKeymapCapturing(true)

        expect(notifications).toBe(0)
    })

    test('구독자는 알림 시점에 이미 새 스냅샷을 읽는다', () => {
        const observed: boolean[] = []
        subscribe(() => observed.push(getKeymapCapturingSnapshot()))

        setKeymapCapturing(true)
        setKeymapCapturing(false)

        expect(observed).toEqual([true, false])
    })

    test('여러 구독자가 모두 알림을 받는다', () => {
        const observed: string[] = []
        subscribe(() => observed.push('first'))
        subscribe(() => observed.push('second'))

        setKeymapCapturing(true)

        expect(observed).toEqual(['first', 'second'])
    })

    test('해제한 구독자는 더 이상 알림을 받지 않는다', () => {
        let notifications = 0
        const release = subscribe(() => {
            notifications += 1
        })

        release()
        setKeymapCapturing(true)

        expect(notifications).toBe(0)
        expect(getKeymapCapturingSnapshot()).toBe(true)
    })

    test('같은 구독자를 두 번 등록해도 Set 이라 한 번만 호출된다', () => {
        let notifications = 0
        const listener = () => {
            notifications += 1
        }
        subscribe(listener)
        subscribe(listener)

        setKeymapCapturing(true)

        expect(notifications).toBe(1)
    })
})
