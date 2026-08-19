import { describe, expect, mock, test } from 'bun:test'

/**
 * `@shared/lib/monaco/setup` pulls in real monaco-editor worker bundles that only Vite's dev/build
 * pipeline can resolve — `bun test` cannot load them (same constraint `lsp-session-registry.test.ts`
 * documents). Stubbing it via `mock.module`, then reaching the module under test through a
 * *dynamic* `import()`, is what makes this file able to load `use-monaco-markers.ts` at all.
 */
type MarkerChangeListener = () => void

const createFakeMonaco = () => {
    let markerChangeListener: MarkerChangeListener | undefined
    let onDidChangeMarkersCallCount = 0
    let snapshot: unknown[] = []

    const monaco = {
        editor: {
            getModelMarkers: () => snapshot,
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
