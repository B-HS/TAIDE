import { describe, expect, mock, test } from 'bun:test'
import { act, renderHook } from '@shared/testing/render'

/**
 * `@shared/lib/monaco/setup` pulls in real monaco-editor worker bundles that only Vite's dev/build
 * pipeline can resolve — `bun test` cannot load them (same constraint `lsp-session-registry.test.ts`
 * documents). Stubbing it via `mock.module`, then reaching the module under test through a
 * *dynamic* `import()`, is what makes this file able to load `use-monaco-markers.ts` at all.
 */
type MarkerChangeListener = () => void

const FAKE_MARKER_SEVERITY = { Hint: 1, Info: 2, Warning: 4, Error: 8 } as const

const createFakeMonaco = () => {
    let markerChangeListener: MarkerChangeListener | undefined
    let onDidChangeMarkersCallCount = 0
    let getModelMarkersCallCount = 0
    let snapshot: unknown[] = []

    const monaco = {
        MarkerSeverity: FAKE_MARKER_SEVERITY,
        editor: {
            getModelMarkers: () => {
                getModelMarkersCallCount += 1
                return snapshot
            },
            onDidChangeMarkers: (listener: MarkerChangeListener) => {
                onDidChangeMarkersCallCount += 1
                markerChangeListener = listener
                return { dispose: () => (markerChangeListener = undefined) }
            },
        },
    }

    return {
        monaco,
        setSnapshot: (next: unknown[]) => (snapshot = next),
        fireMarkersChanged: () => markerChangeListener?.(),
        isSubscribed: () => markerChangeListener !== undefined,
        getOnDidChangeMarkersCallCount: () => onDidChangeMarkersCallCount,
        getModelMarkersCallCount: () => getModelMarkersCallCount,
    }
}

const fakeMonacoModule = createFakeMonaco()

mock.module('@shared/lib/monaco/setup', () => ({ monaco: fakeMonacoModule.monaco }))

const importUseMonacoMarkers = () => import('@shared/hooks/use-monaco-markers')

describe('subscribeToMonacoMarkers — 마운트 수명 구독 (F7#17)', () => {
    test('첫 구독자가 붙을 때만 monaco.onDidChangeMarkers 를 구독한다', async () => {
        const { subscribeToMonacoMarkers } = await importUseMonacoMarkers()
        expect(fakeMonacoModule.isSubscribed()).toBe(false)

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        expect(fakeMonacoModule.isSubscribed()).toBe(true)

        unsubscribe()
    })

    test('마지막 구독자가 해제되면 monaco 구독도 함께 해제된다', async () => {
        const { subscribeToMonacoMarkers } = await importUseMonacoMarkers()
        const unsubscribeA = subscribeToMonacoMarkers(() => {})
        const unsubscribeB = subscribeToMonacoMarkers(() => {})

        unsubscribeA()
        expect(fakeMonacoModule.isSubscribed()).toBe(true)

        unsubscribeB()
        expect(fakeMonacoModule.isSubscribed()).toBe(false)
    })

    test('활성 구독자가 있는 동안 추가 구독자가 붙어도 monaco.onDidChangeMarkers 를 다시 구독하지 않는다', async () => {
        const { subscribeToMonacoMarkers } = await importUseMonacoMarkers()
        const before = fakeMonacoModule.getOnDidChangeMarkersCallCount()

        const unsubscribeA = subscribeToMonacoMarkers(() => {})
        expect(fakeMonacoModule.getOnDidChangeMarkersCallCount()).toBe(before + 1)

        const unsubscribeB = subscribeToMonacoMarkers(() => {})
        expect(fakeMonacoModule.getOnDidChangeMarkersCallCount()).toBe(before + 1)

        unsubscribeA()
        unsubscribeB()
    })

    test('재구독 시 최신 snapshot 을 즉시 반영한다', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkersSnapshot } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([{ message: 'stale, set before any subscriber' }])

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        expect(getMonacoMarkersSnapshot() as unknown[]).toEqual([{ message: 'stale, set before any subscriber' }])

        unsubscribe()
    })

    test('marker 변경 발화 시 모든 구독자에게 통지하고 snapshot 을 갱신한다', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkersSnapshot } = await importUseMonacoMarkers()
        let notifiedA = false
        let notifiedB = false
        const unsubscribeA = subscribeToMonacoMarkers(() => (notifiedA = true))
        const unsubscribeB = subscribeToMonacoMarkers(() => (notifiedB = true))

        fakeMonacoModule.setSnapshot([{ message: 'new marker' }])
        fakeMonacoModule.fireMarkersChanged()

        expect(notifiedA).toBe(true)
        expect(notifiedB).toBe(true)
        expect(getMonacoMarkersSnapshot() as unknown[]).toEqual([{ message: 'new marker' }])

        unsubscribeA()
        unsubscribeB()
    })
})

describe('getMonacoMarkerCountsSnapshot — severity 카운트 티어 (research 3a H2)', () => {
    test('severity 별 합계를 내고 없는 severity 는 0 으로 채운다', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkerCountsSnapshot } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([
            { severity: FAKE_MARKER_SEVERITY.Error, message: 'a' },
            { severity: FAKE_MARKER_SEVERITY.Error, message: 'b' },
            { severity: FAKE_MARKER_SEVERITY.Warning, message: 'c' },
        ])

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        const counts = getMonacoMarkerCountsSnapshot()

        expect(counts[FAKE_MARKER_SEVERITY.Error]).toBe(2)
        expect(counts[FAKE_MARKER_SEVERITY.Warning]).toBe(1)
        expect(counts[FAKE_MARKER_SEVERITY.Info]).toBe(0)
        expect(counts[FAKE_MARKER_SEVERITY.Hint]).toBe(0)

        unsubscribe()
    })

    test('합계가 그대로인 marker 변경에서는 같은 참조를 돌려준다 (useSyncExternalStore 무한 렌더 회귀 방지)', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkerCountsSnapshot } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Error, message: 'before' }])

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        const before = getMonacoMarkerCountsSnapshot()

        fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Error, message: 'after' }])
        fakeMonacoModule.fireMarkersChanged()

        expect(getMonacoMarkerCountsSnapshot()).toBe(before)

        unsubscribe()
    })

    test('합계가 바뀌면 새 참조를 돌려준다', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkerCountsSnapshot } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Error, message: 'only' }])

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        const before = getMonacoMarkerCountsSnapshot()

        fakeMonacoModule.setSnapshot([
            { severity: FAKE_MARKER_SEVERITY.Error, message: 'only' },
            { severity: FAKE_MARKER_SEVERITY.Info, message: 'added' },
        ])
        fakeMonacoModule.fireMarkersChanged()

        const after = getMonacoMarkerCountsSnapshot()
        expect(after).not.toBe(before)
        expect(after[FAKE_MARKER_SEVERITY.Info]).toBe(1)
        expect(before[FAKE_MARKER_SEVERITY.Info]).toBe(0)

        unsubscribe()
    })

    test('두 티어가 monaco 를 한 번만 읽고 함께 갱신된다', async () => {
        const { subscribeToMonacoMarkers, getMonacoMarkersSnapshot, getMonacoMarkerCountsSnapshot } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([])

        const unsubscribe = subscribeToMonacoMarkers(() => {})
        fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Warning, message: 'both tiers' }])
        fakeMonacoModule.fireMarkersChanged()

        expect(getMonacoMarkersSnapshot() as unknown[]).toEqual([{ severity: FAKE_MARKER_SEVERITY.Warning, message: 'both tiers' }])
        expect(getMonacoMarkerCountsSnapshot()[FAKE_MARKER_SEVERITY.Warning]).toBe(1)

        unsubscribe()
    })

    test('합계가 그대로인 marker 변경에서는 훅 소비자가 다시 렌더되지 않는다', async () => {
        const { useMonacoMarkerCounts } = await importUseMonacoMarkers()
        fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Error, message: 'before' }])

        let renderCount = 0
        const { result, unmount } = renderHook(() => {
            renderCount += 1
            return useMonacoMarkerCounts()
        })
        const rendersAfterMount = renderCount
        const first = result.current

        act(() => {
            fakeMonacoModule.setSnapshot([{ severity: FAKE_MARKER_SEVERITY.Error, message: 'after' }])
            fakeMonacoModule.fireMarkersChanged()
        })

        expect(renderCount).toBe(rendersAfterMount)
        expect(result.current).toBe(first)

        act(() => {
            fakeMonacoModule.setSnapshot([
                { severity: FAKE_MARKER_SEVERITY.Error, message: 'after' },
                { severity: FAKE_MARKER_SEVERITY.Error, message: 'one more' },
            ])
            fakeMonacoModule.fireMarkersChanged()
        })

        expect(renderCount).toBeGreaterThan(rendersAfterMount)
        expect(result.current[FAKE_MARKER_SEVERITY.Error]).toBe(2)

        unmount()
    })

    test('구독자가 몇이든 marker 변경 1회당 getModelMarkers 호출은 1회다 (연산 예산)', async () => {
        const { subscribeToMonacoMarkers } = await importUseMonacoMarkers()
        const unsubscribeA = subscribeToMonacoMarkers(() => {})
        const unsubscribeB = subscribeToMonacoMarkers(() => {})
        const unsubscribeC = subscribeToMonacoMarkers(() => {})
        const before = fakeMonacoModule.getModelMarkersCallCount()

        fakeMonacoModule.fireMarkersChanged()
        fakeMonacoModule.fireMarkersChanged()

        expect(fakeMonacoModule.getModelMarkersCallCount() - before).toBe(2)

        unsubscribeA()
        unsubscribeB()
        unsubscribeC()
    })
})
