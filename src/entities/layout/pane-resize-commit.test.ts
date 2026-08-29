import { beforeEach, describe, expect, test } from 'bun:test'
import type { PaneResizeCommitScheduler } from '@entities/layout/pane-resize-commit'
import { clearPendingPaneResizeCommits, PANE_RESIZE_COMMIT_DEBOUNCE_MS, schedulePaneResizeCommit } from '@entities/layout/pane-resize-commit'

const createFakeScheduler = () => {
    const callbacks = new Map<number, () => void>()
    const cancelled: number[] = []
    let nextId = 1

    const scheduler: PaneResizeCommitScheduler = {
        schedule: (callback, delayMs) => {
            const id = nextId++
            callbacks.set(id, callback)
            scheduledDelays.push(delayMs)
            return id
        },
        cancel: (timerId) => {
            const id = timerId as number
            cancelled.push(id)
            callbacks.delete(id)
        },
    }
    const scheduledDelays: number[] = []

    return {
        scheduler,
        cancelled,
        scheduledDelays,
        runAll: () => {
            const pending = [...callbacks.values()]
            callbacks.clear()
            pending.forEach((callback) => callback())
        },
    }
}

describe('schedulePaneResizeCommit', () => {
    beforeEach(clearPendingPaneResizeCommits)

    test('타이머가 만료되어야 커밋한다 — leading edge 없이 trailing 전용이다', () => {
        const fake = createFakeScheduler()
        const committed: string[] = []

        schedulePaneResizeCommit('p:1', () => committed.push('first'), { scheduler: fake.scheduler })

        expect(committed).toEqual([])
        fake.runAll()
        expect(committed).toEqual(['first'])
    })

    test('같은 pane 의 연속 호출은 직전 타이머를 취소하고 마지막 커밋 1회만 남긴다 (키 오토리핏 흡수)', () => {
        const fake = createFakeScheduler()
        const committed: string[] = []

        schedulePaneResizeCommit('p:1', () => committed.push('a'), { scheduler: fake.scheduler })
        schedulePaneResizeCommit('p:1', () => committed.push('b'), { scheduler: fake.scheduler })
        schedulePaneResizeCommit('p:1', () => committed.push('c'), { scheduler: fake.scheduler })
        fake.runAll()

        expect(committed).toEqual(['c'])
        expect(fake.cancelled.length).toBe(2)
    })

    test('pane 키가 다르면 서로 취소하지 않고 각각 커밋한다', () => {
        const fake = createFakeScheduler()
        const committed: string[] = []

        schedulePaneResizeCommit('p:1', () => committed.push('one'), { scheduler: fake.scheduler })
        schedulePaneResizeCommit('p:2', () => committed.push('two'), { scheduler: fake.scheduler })
        fake.runAll()

        expect(committed.toSorted()).toEqual(['one', 'two'])
        expect(fake.cancelled).toEqual([])
    })

    test('커밋이 끝난 뒤의 새 호출은 취소할 타이머가 없다 — 만료된 항목을 남겨두지 않는다', () => {
        const fake = createFakeScheduler()

        schedulePaneResizeCommit('p:1', () => undefined, { scheduler: fake.scheduler })
        fake.runAll()
        schedulePaneResizeCommit('p:1', () => undefined, { scheduler: fake.scheduler })

        expect(fake.cancelled).toEqual([])
    })

    test('기본 지연은 PANE_RESIZE_COMMIT_DEBOUNCE_MS 다', () => {
        const fake = createFakeScheduler()

        schedulePaneResizeCommit('p:1', () => undefined, { scheduler: fake.scheduler })

        expect(fake.scheduledDelays).toEqual([PANE_RESIZE_COMMIT_DEBOUNCE_MS])
    })
})
