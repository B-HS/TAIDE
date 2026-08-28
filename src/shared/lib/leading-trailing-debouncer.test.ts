import { describe, expect, test } from 'bun:test'
import { createLeadingTrailingDebouncer, type LeadingTrailingDebouncerScheduler } from '@shared/lib/leading-trailing-debouncer'

const FAKE_DEBOUNCE_DELAY_MS = 150

const createFakeTimerScheduler = () => {
    let nextTimerId = 1
    const scheduled = new Map<number, { callback: () => void; dueAt: number }>()
    let now = 0
    const scheduler: LeadingTrailingDebouncerScheduler<number> = {
        schedule: (callback, delayMs) => {
            const timerId = nextTimerId++
            scheduled.set(timerId, { callback, dueAt: now + delayMs })
            return timerId
        },
        cancel: (timerId) => {
            scheduled.delete(timerId)
        },
    }
    const advance = (ms: number) => {
        now += ms
        const due = Array.from(scheduled.entries()).filter(([, entry]) => entry.dueAt <= now)
        for (const [timerId, entry] of due) {
            scheduled.delete(timerId)
            entry.callback()
        }
    }
    return { scheduler, advance }
}

describe('createLeadingTrailingDebouncer', () => {
    test('유휴 상태의 첫 호출은 즉시 실행된다', () => {
        const runs: number[] = []
        const { scheduler } = createFakeTimerScheduler()
        const debouncer = createLeadingTrailingDebouncer(() => runs.push(1), FAKE_DEBOUNCE_DELAY_MS, scheduler)

        debouncer.trigger()

        expect(runs).toEqual([1])
    })

    test('지연시간 내의 연속 호출은 트레일링으로 한 번만, 지연 이후에 실행된다', () => {
        const runs: number[] = []
        const { scheduler, advance } = createFakeTimerScheduler()
        const debouncer = createLeadingTrailingDebouncer(() => runs.push(runs.length + 1), FAKE_DEBOUNCE_DELAY_MS, scheduler)

        debouncer.trigger()
        advance(10)
        debouncer.trigger()
        advance(10)
        debouncer.trigger()
        expect(runs).toEqual([1])

        advance(FAKE_DEBOUNCE_DELAY_MS)
        expect(runs).toEqual([1, 2])
    })

    test('연속 호출마다 대기 창을 리셋해, 계속 호출되는 동안에는 트레일링이 실행되지 않는다', () => {
        const runs: number[] = []
        const { scheduler, advance } = createFakeTimerScheduler()
        const debouncer = createLeadingTrailingDebouncer(() => runs.push(runs.length + 1), FAKE_DEBOUNCE_DELAY_MS, scheduler)

        debouncer.trigger()
        for (let i = 0; i < 10; i += 1) {
            advance(FAKE_DEBOUNCE_DELAY_MS - 10)
            debouncer.trigger()
        }
        expect(runs).toEqual([1])

        advance(FAKE_DEBOUNCE_DELAY_MS)
        expect(runs).toEqual([1, 2])
    })

    test('트레일링 실행 후에는 다시 유휴 상태가 되어 다음 호출이 즉시 실행된다', () => {
        const runs: number[] = []
        const { scheduler, advance } = createFakeTimerScheduler()
        const debouncer = createLeadingTrailingDebouncer(() => runs.push(runs.length + 1), FAKE_DEBOUNCE_DELAY_MS, scheduler)

        debouncer.trigger()
        debouncer.trigger()
        advance(FAKE_DEBOUNCE_DELAY_MS)
        expect(runs).toEqual([1, 2])

        debouncer.trigger()
        expect(runs).toEqual([1, 2, 3])
    })

    test('trigger 는 유휴 상태에서 실행한 run 의 반환값을 그대로 돌려주고, 트레일링으로 접힌 호출은 undefined 를 돌려준다', () => {
        const { scheduler } = createFakeTimerScheduler()
        const debouncer = createLeadingTrailingDebouncer(() => 'leading-result', FAKE_DEBOUNCE_DELAY_MS, scheduler)

        expect(debouncer.trigger()).toBe('leading-result')
        expect(debouncer.trigger()).toBeUndefined()
    })
})
