import { describe, expect, test } from 'bun:test'
import { createFrameCoalescer, type FrameCoalescerScheduler } from '@shared/lib/frame-coalescer'

const createFakeFrameScheduler = () => {
    let nextFrameId = 1
    const scheduled = new Map<number, () => void>()
    const scheduler: FrameCoalescerScheduler = {
        requestFrame: (callback) => {
            const frameId = nextFrameId++
            scheduled.set(frameId, callback)
            return frameId
        },
        cancelFrame: (frameId) => {
            scheduled.delete(frameId)
        },
    }
    const runFrame = () => {
        const entries = Array.from(scheduled.entries())
        scheduled.clear()
        for (const [, callback] of entries) callback()
    }
    return { scheduler, runFrame, pendingFrameCount: () => scheduled.size }
}

describe('createFrameCoalescer', () => {
    test('한 프레임 안의 연속 push 는 flush 를 한 번만, 최신 값으로 호출한다', () => {
        const flushed: number[] = []
        const { scheduler, runFrame } = createFakeFrameScheduler()
        const coalescer = createFrameCoalescer<number>((value) => flushed.push(value), scheduler)

        coalescer.push(1)
        coalescer.push(2)
        coalescer.push(3)
        runFrame()

        expect(flushed).toEqual([3])
    })

    test('flush 이후 새 push 는 다음 프레임에 다시 flush 된다', () => {
        const flushed: number[] = []
        const { scheduler, runFrame } = createFakeFrameScheduler()
        const coalescer = createFrameCoalescer<number>((value) => flushed.push(value), scheduler)

        coalescer.push(1)
        runFrame()
        coalescer.push(2)
        runFrame()

        expect(flushed).toEqual([1, 2])
    })

    test('플러시된 프레임 이후 추가 push 가 없으면 다음 프레임은 flush 를 다시 호출하지 않는다', () => {
        const flushed: number[] = []
        const { scheduler, runFrame } = createFakeFrameScheduler()
        const coalescer = createFrameCoalescer<number>((value) => flushed.push(value), scheduler)

        coalescer.push(1)
        runFrame()
        runFrame()

        expect(flushed).toEqual([1])
    })

    test('cancel 은 예약된 프레임과 보류 값을 모두 지워, 늦게 도착한 프레임이 값을 적용하지 못한다', () => {
        const flushed: number[] = []
        const { scheduler, runFrame, pendingFrameCount } = createFakeFrameScheduler()
        const coalescer = createFrameCoalescer<number>((value) => flushed.push(value), scheduler)

        coalescer.push(1)
        coalescer.cancel()
        runFrame()

        expect(flushed).toEqual([])
        expect(pendingFrameCount()).toBe(0)
    })

    test('cancel 이후 다시 push 하면 정상적으로 새 프레임을 예약한다', () => {
        const flushed: number[] = []
        const { scheduler, runFrame } = createFakeFrameScheduler()
        const coalescer = createFrameCoalescer<number>((value) => flushed.push(value), scheduler)

        coalescer.push(1)
        coalescer.cancel()
        coalescer.push(2)
        runFrame()

        expect(flushed).toEqual([2])
    })
})
