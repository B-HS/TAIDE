import { afterEach, beforeEach, describe, expect, test } from 'bun:test'
import type { FC } from 'react'
import { useRef } from 'react'
import { render, screen } from '@shared/testing/render'
import { useOverlayScrollbar } from '@shared/hooks/use-overlay-scrollbar'

/**
 * happy-dom ships a `ResizeObserver` whose `observe`/`unobserve` are empty stubs that keep no
 * target list (`node_modules/happy-dom/lib/resize-observer/ResizeObserver.js`), so the only way to
 * assert *which* elements this hook observes is to stand in for the constructor. Swapped per test
 * and restored afterwards, since bun shares globals across test files.
 */
type ObserverCall = { method: 'observe' | 'unobserve'; target: Element }

const observerCalls: ObserverCall[] = []

class RecordingResizeObserver {
    observe(target: Element) {
        observerCalls.push({ method: 'observe', target })
    }

    unobserve(target: Element) {
        observerCalls.push({ method: 'unobserve', target })
    }

    disconnect() {}
}

const nativeResizeObserver = globalThis.ResizeObserver

const observedTargets = () => observerCalls.filter((call) => call.method === 'observe').map((call) => call.target)

const ScrollbarHarness: FC<{ rowIds: string[] }> = ({ rowIds }) => {
    const viewportRef = useRef<HTMLDivElement>(null)
    const { trackRef, thumbRef } = useOverlayScrollbar({ viewportRef })

    return (
        <div>
            <div ref={viewportRef} data-testid='viewport'>
                {rowIds.map((id) => (
                    <div key={id} data-testid={`row-${id}`} />
                ))}
            </div>
            <div ref={trackRef} data-testid='track'>
                <div ref={thumbRef} />
            </div>
        </div>
    )
}

const appendRow = (viewport: HTMLElement) => {
    const row = viewport.ownerDocument.createElement('div')
    viewport.appendChild(row)
    return row
}

/**
 * happy-dom delivers `MutationObserver` records through `queueMicrotask`
 * (`node_modules/happy-dom/lib/mutation-observer/MutationObserverListener.js`), so draining the
 * microtask queue is enough — and is used instead of `@testing-library`'s `waitFor`, whose polling
 * never settles under bun + happy-dom (the run hangs rather than failing).
 */
const flushMutationRecords = async () => {
    await Promise.resolve()
    await Promise.resolve()
}

beforeEach(() => {
    observerCalls.length = 0
    globalThis.ResizeObserver = RecordingResizeObserver
})

afterEach(() => {
    globalThis.ResizeObserver = nativeResizeObserver
})

describe('useOverlayScrollbar — 콘텐츠 관찰 대상 (research 3a L3)', () => {
    test('마운트 시 뷰포트와 직속 자식 전부를 관찰한다', () => {
        render(<ScrollbarHarness rowIds={['a', 'b', 'c']} />)

        expect(observedTargets()).toEqual([
            screen.getByTestId('viewport'),
            screen.getByTestId('row-a'),
            screen.getByTestId('row-b'),
            screen.getByTestId('row-c'),
        ])
    })

    test('자식이 추가되면 추가된 자식만 관찰하고 기존 자식을 다시 관찰하지 않는다', async () => {
        render(<ScrollbarHarness rowIds={['a', 'b', 'c']} />)
        const viewport = screen.getByTestId('viewport')
        observerCalls.length = 0

        const added = appendRow(viewport)
        await flushMutationRecords()

        expect(observerCalls).toEqual([{ method: 'observe', target: added }])
    })

    test('여러 자식이 한 번에 추가돼도 관찰 호출은 추가된 수만큼만 늘어난다', async () => {
        render(<ScrollbarHarness rowIds={['a', 'b', 'c']} />)
        const viewport = screen.getByTestId('viewport')
        observerCalls.length = 0

        const first = appendRow(viewport)
        const second = appendRow(viewport)
        await flushMutationRecords()

        expect(observedTargets()).toEqual([first, second])
    })

    test('자식이 제거되면 그 자식만 관찰 해제한다', async () => {
        render(<ScrollbarHarness rowIds={['a', 'b', 'c']} />)
        const viewport = screen.getByTestId('viewport')
        const removed = screen.getByTestId('row-b')
        observerCalls.length = 0

        viewport.removeChild(removed)
        await flushMutationRecords()

        expect(observerCalls).toEqual([{ method: 'unobserve', target: removed }])
    })

    test('재정렬로 같은 자식이 제거·재추가돼도 마지막에 관찰 상태로 남는다', async () => {
        render(<ScrollbarHarness rowIds={['a', 'b', 'c']} />)
        const viewport = screen.getByTestId('viewport')
        const moved = screen.getByTestId('row-c')
        observerCalls.length = 0

        viewport.insertBefore(moved, viewport.firstChild)
        await flushMutationRecords()

        expect(observerCalls.filter((call) => call.target === moved).at(-1)?.method).toBe('observe')
    })

    test('텍스트 노드가 추가돼도 ResizeObserver 를 건드리지 않는다', async () => {
        render(<ScrollbarHarness rowIds={['a']} />)
        const viewport = screen.getByTestId('viewport')
        observerCalls.length = 0

        viewport.appendChild(viewport.ownerDocument.createTextNode('가상화 sizing 텍스트'))
        const added = appendRow(viewport)
        await flushMutationRecords()

        expect(observerCalls).toEqual([{ method: 'observe', target: added }])
    })
})
