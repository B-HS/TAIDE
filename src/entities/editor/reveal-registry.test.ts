import { describe, expect, mock, test } from 'bun:test'

/**
 * `reveal-registry.ts` imports `@shared/lib/monaco/setup`, which pulls in real monaco-editor
 * worker bundles (`?worker` imports) that only Vite's dev/build pipeline can resolve — `bun test`
 * cannot load them at all. Stubbing `@shared/lib/monaco/setup`, then reaching the module under test
 * through a *dynamic* `import()` (not a static import — Bun resolves the whole static import graph,
 * including the offending worker files, before a same-file `mock.module` call would ever run) is
 * what makes this file able to load `reveal-registry.ts` at all (same workaround
 * `ide-sync-provider.test.ts` documents).
 */
type FakeEditor = {
    calls: { setPosition: { lineNumber: number; column: number }[]; revealPositionInCenter: { lineNumber: number; column: number }[]; focus: number }
    getModel: () => { uri: { toString: () => string } } | null
    setPosition: (position: { lineNumber: number; column: number }) => void
    revealPositionInCenter: (position: { lineNumber: number; column: number }) => void
    focus: () => void
}

const createFakeEditor = (uriString: string | null): FakeEditor => {
    const calls: FakeEditor['calls'] = { setPosition: [], revealPositionInCenter: [], focus: 0 }
    return {
        calls,
        getModel: () => (uriString ? { uri: { toString: () => uriString } } : null),
        setPosition: (position) => calls.setPosition.push(position),
        revealPositionInCenter: (position) => calls.revealPositionInCenter.push(position),
        focus: () => {
            calls.focus += 1
        },
    }
}

let fakeEditors: FakeEditor[] = []

const FAKE_MONACO = {
    Uri: { file: (path: string) => ({ toString: () => `file://${path}` }) },
    editor: { getEditors: () => fakeEditors },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

const importRevealRegistry = () => import('@entities/editor/reveal-registry')

const SHORT_TTL_MS = 20

describe('requestReveal / 대상 에디터가 이미 열려 있는 경우', () => {
    test('즉시 커서를 이동시키고 대기열에 쌓지 않는다', async () => {
        const { requestReveal, consumePendingReveal } = await importRevealRegistry()
        const path = '/repo/already-open.ts'
        const editor = createFakeEditor(`file://${path}`)
        fakeEditors = [editor]

        requestReveal(path, 3, 5)

        expect(editor.calls.setPosition).toEqual([{ lineNumber: 3, column: 5 }])
        expect(editor.calls.revealPositionInCenter).toEqual([{ lineNumber: 3, column: 5 }])
        expect(editor.calls.focus).toBe(1)

        const laterEditor = createFakeEditor(`file://${path}`)
        consumePendingReveal(path, laterEditor as unknown as Parameters<typeof consumePendingReveal>[1])
        expect(laterEditor.calls.setPosition).toEqual([])
    })
})

describe('requestReveal / 대상 에디터가 아직 없는 경우', () => {
    test('나중에 그 경로의 에디터가 마운트되면 보류된 위치를 적용한다', async () => {
        const { requestReveal, consumePendingReveal } = await importRevealRegistry()
        const path = '/repo/not-yet-open.ts'
        fakeEditors = []

        requestReveal(path, 10, 2)

        const editor = createFakeEditor(`file://${path}`)
        consumePendingReveal(path, editor as unknown as Parameters<typeof consumePendingReveal>[1])

        expect(editor.calls.setPosition).toEqual([{ lineNumber: 10, column: 2 }])
    })

    test('같은 경로를 다시 요청하면 가장 최근 요청의 위치만 적용된다', async () => {
        const { requestReveal, consumePendingReveal } = await importRevealRegistry()
        const path = '/repo/re-requested.ts'
        fakeEditors = []

        requestReveal(path, 1, 1)
        requestReveal(path, 99, 4)

        const editor = createFakeEditor(`file://${path}`)
        consumePendingReveal(path, editor as unknown as Parameters<typeof consumePendingReveal>[1])

        expect(editor.calls.setPosition).toEqual([{ lineNumber: 99, column: 4 }])
    })

    test('TTL 이 지나면 보류된 요청이 폐기되어 나중에 열려도 커서를 옮기지 않는다', async () => {
        const { requestReveal, consumePendingReveal } = await importRevealRegistry()
        const path = '/repo/expired.ts'
        fakeEditors = []

        requestReveal(path, 7, 1, SHORT_TTL_MS)
        await new Promise((resolve) => setTimeout(resolve, SHORT_TTL_MS * 3))

        const editor = createFakeEditor(`file://${path}`)
        consumePendingReveal(path, editor as unknown as Parameters<typeof consumePendingReveal>[1])

        expect(editor.calls.setPosition).toEqual([])
    })

    test('만료 전에는 정상적으로 적용된다', async () => {
        const { requestReveal, consumePendingReveal } = await importRevealRegistry()
        const path = '/repo/not-expired.ts'
        fakeEditors = []

        requestReveal(path, 7, 1, SHORT_TTL_MS * 10)

        const editor = createFakeEditor(`file://${path}`)
        consumePendingReveal(path, editor as unknown as Parameters<typeof consumePendingReveal>[1])

        expect(editor.calls.setPosition).toEqual([{ lineNumber: 7, column: 1 }])
    })
})

describe('consumePendingReveal / 보류된 요청이 없는 경우', () => {
    test('아무 것도 하지 않는다', async () => {
        const { consumePendingReveal } = await importRevealRegistry()
        const editor = createFakeEditor('file:///repo/none.ts')

        consumePendingReveal('/repo/none.ts', editor as unknown as Parameters<typeof consumePendingReveal>[1])

        expect(editor.calls.setPosition).toEqual([])
    })
})
