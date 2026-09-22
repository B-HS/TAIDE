import { describe, expect, mock, spyOn, test } from 'bun:test'
import type { PaneNode, ProjectLayout, Settings } from '@shared/api/bindings'
import * as layoutIpc from '@entities/layout/layout.ipc'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * Pane focus follows the pointer into a pane's *body*, not just into its tab bar (d-65 contract
 * §1 F2). The three outcomes worth locking are the ones the guard exists for: a press inside an
 * unfocused pane claims focus, the `focusin` the browser fires for that same press does not claim
 * it a second time, and a press inside the pane that is already focused says nothing at all.
 *
 * `welcomeOnEmptyEditor: false` is seeded so each tabless leaf renders the plain `editor.noFileOpen`
 * line — a body that is present synchronously, with no lazy chunk and no Suspense boundary between
 * the event target and the wrapper under test. `zen` keeps the tab bar (dnd-kit, and the very path
 * this change deletes) out of the tree, so the press being asserted is unambiguously a body press.
 *
 * `PaneNodeView` statically imports every pane body, monaco included, so `@shared/lib/monaco/setup`
 * is stubbed before it is pulled in through a *dynamic* `import()`
 * (`docs/memory/test-conventions.md` §3). The IPC itself is a `spyOn` rather than a `mock.module`
 * so it is undone by the preload's `beforeEach(mock.restore)` instead of leaking into every other
 * file's `layout.ipc`.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: {
        Uri: { file: () => ({ toString: () => '' }) },
        editor: {},
        languages: { InlineCompletionTriggerKind: { Automatic: 0, Explicit: 1 } },
    },
}))

const importPaneNodeView = () => import('@widgets/editor-area/pane-node-view')

const PROJECT_ID = 'project-1'
const LEFT_PANE_ID = 'leaf-left'
const RIGHT_PANE_ID = 'leaf-right'
const EQUAL_SPLIT = 50

const SETTINGS: Partial<Settings> = { welcomeOnEmptyEditor: false }

const SPLIT_ROOT: PaneNode = {
    node: 'split',
    id: 'split-root',
    dir: 'horizontal',
    sizes: [EQUAL_SPLIT, EQUAL_SPLIT],
    children: [
        { node: 'leaf', id: LEFT_PANE_ID, tabs: [], active: null },
        { node: 'leaf', id: RIGHT_PANE_ID, tabs: [], active: null },
    ],
}

const focusedLayout = (focusedPane: string): ProjectLayout => ({ version: 2, root: SPLIT_ROOT, focusedPane, revision: 2 })

/** Drains the mutation's own promise chain (retryer hop, `onSettled`, the notify-manager microtask) so its state lands inside the `act` scope the events were fired in. */
const settleMutation = () => new Promise((resolve) => setTimeout(resolve, 0))

const spyOnFocusPane = (focusedPane: string) => spyOn(layoutIpc, 'focusPane').mockResolvedValue(focusedLayout(focusedPane))

/** `viewAt` comes back alongside the render result so a case can move the focused pane the way the server would — a re-render with a new `focusedPaneId` — instead of remounting and losing the guard under test. */
const renderSplit = async () => {
    const { PaneNodeView } = await importPaneNodeView()
    const queryClient = createTestQueryClient()
    queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, SETTINGS)
    const viewAt = (focusedPaneId: string) => (
        <PaneNodeView node={SPLIT_ROOT} projectId={PROJECT_ID} focusedPaneId={focusedPaneId} isDragging={false} overTarget={null} zen={true} />
    )

    return { viewAt, ...renderWithProviders(viewAt(LEFT_PANE_ID), { queryClient }) }
}

const paneBodyOf = (side: 'left' | 'right') => screen.getAllByText('editor.noFileOpen')[side === 'left' ? 0 : 1]

describe('PaneNodeView pane 포커스 추종', () => {
    test('포커스가 아닌 pane 안을 누르면 그 pane 으로 포커스 IPC 를 한 번 보낸다', async () => {
        const focusPane = spyOnFocusPane(RIGHT_PANE_ID)
        await renderSplit()

        await act(async () => {
            fireEvent.pointerDown(paneBodyOf('right'))
            await settleMutation()
        })

        expect(focusPane).toHaveBeenCalledTimes(1)
        expect(focusPane.mock.calls.at(-1)?.[0]).toBe(RIGHT_PANE_ID)
    })

    test('같은 클릭이 잇따라 내는 focusin 은 IPC 를 다시 보내지 않는다', async () => {
        const focusPane = spyOnFocusPane(RIGHT_PANE_ID)
        await renderSplit()

        const body = paneBodyOf('right')
        await act(async () => {
            fireEvent.pointerDown(body)
            fireEvent.focusIn(body)
            await settleMutation()
        })

        expect(focusPane).toHaveBeenCalledTimes(1)
    })

    test('이미 포커스된 pane 안을 눌러도 IPC 를 보내지 않는다', async () => {
        const focusPane = spyOnFocusPane(LEFT_PANE_ID)
        await renderSplit()

        const body = paneBodyOf('left')
        await act(async () => {
            fireEvent.pointerDown(body)
            fireEvent.focusIn(body)
            await settleMutation()
        })

        expect(focusPane).toHaveBeenCalledTimes(0)
    })

    test('포커스가 다른 pane 을 거쳐 돌아온 뒤 다시 누르면 가드가 막지 않고 또 보낸다', async () => {
        const focusPane = spyOnFocusPane(RIGHT_PANE_ID)
        const { viewAt, rerender } = await renderSplit()

        await act(async () => {
            fireEvent.pointerDown(paneBodyOf('right'))
            await settleMutation()
        })
        await act(async () => {
            rerender(viewAt(RIGHT_PANE_ID))
            await settleMutation()
        })
        await act(async () => {
            rerender(viewAt(LEFT_PANE_ID))
            await settleMutation()
        })
        await act(async () => {
            fireEvent.pointerDown(paneBodyOf('right'))
            await settleMutation()
        })

        expect(focusPane).toHaveBeenCalledTimes(2)
    })
})
