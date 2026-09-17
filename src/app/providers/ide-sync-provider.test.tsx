import { afterAll, beforeAll, describe, expect, mock, spyOn, test } from 'bun:test'
import type { EventCallback } from '@tauri-apps/api/event'
import type { IdeSaveRequested, PaneNode, ProjectLayout, Tab } from '@shared/api/bindings'
import { commands, events } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders } from '@shared/testing/render'

/**
 * `ide-sync-provider.tsx` reaches `@entities/editor/model-registry` (for `getModel`),
 * `@shared/hooks/use-monaco-markers`, and `@features/problems/problem-severity` — all three pull in
 * `@shared/lib/monaco/setup`, which imports real monaco-editor worker bundles (`?worker` imports)
 * that only Vite's dev/build pipeline can resolve. `bun test` cannot load them at all (same
 * constraint `lsp-session-registry.test.ts` documents on its own `mock.module` setup: importing a
 * module that reaches `monaco/setup` fails with "Missing 'default' export ... ts.worker.js?worker"
 * before any test code runs). Stubbing `@shared/lib/monaco/setup`, then reaching the module under
 * test through a *dynamic* `import()` (not a static import — Bun resolves the whole static import
 * graph, including the offending worker files, before a same-file `mock.module` call would ever
 * run) is what makes this file able to load `ide-sync-provider.tsx` at all.
 */
const FAKE_MONACO = {
    Uri: {
        file: (path: string) => ({ toString: () => `file://${path}` }),
        parse: (value: string) => ({ toString: () => value }),
    },
    editor: {
        getModelMarkers: () => [],
        onDidChangeMarkers: () => {},
    },
    /** `use-monaco-markers.ts` builds its per-severity counts object from these at subscribe time. */
    MarkerSeverity: { Hint: 1, Info: 2, Warning: 4, Error: 8 },
}

mock.module('@shared/lib/monaco/setup', () => ({ monaco: FAKE_MONACO }))

describe('IdeSyncProvider 모듈 로드', () => {
    test('monaco worker 를 정적 임포트 그래프에서 우회해 로드된다', async () => {
        const imported = await import('@app/providers/ide-sync-provider')
        expect(typeof imported.IdeSyncProvider).toBe('function')
    })
})

const PROJECT_ID = 'project-1'
const REQUEST_ID = 'request-1'
const DIRTY_PATH = '/repo/dirty.ts'
const CLEAN_PATH = '/repo/clean.ts'
const FAKE_EVENT_ID = 1
const EVENT_PLUGIN_COMMAND_PREFIX = 'plugin:event'
const IPC_UNAVAILABLE_ERROR = { code: 'Unknown', message: 'no backend in the test harness' }
const TAURI_EVENT_PLUGIN_INTERNALS_KEY = '__TAURI_EVENT_PLUGIN_INTERNALS__'

/** Subscribing has to succeed — the provider registers four listeners during mount and `listen()` reaches `transformCallback` synchronously — while real commands fail as they do everywhere else in the harness. */
const fakeInvoke = (command: string) =>
    command.startsWith(EVENT_PLUGIN_COMMAND_PREFIX) ? Promise.resolve(FAKE_EVENT_ID) : Promise.reject(IPC_UNAVAILABLE_ERROR)

const buildFileTab = (id: string, path: string, dirty: boolean): Tab => ({ id, kind: { kind: 'file', path }, title: path, dirty })

const buildLeaf = (id: string, tabs: Tab[]): PaneNode => ({ node: 'leaf', id, tabs, active: tabs[0]?.id ?? null })

const buildLayout = (mainTabs: Tab[], auxiliaryTabs: Tab[]): ProjectLayout => ({
    version: 2,
    root: buildLeaf('main-leaf', mainTabs),
    focusedPane: 'main-leaf',
    auxiliaryWindows: [{ slot: 1, root: buildLeaf('aux-leaf', auxiliaryTabs), focusedPane: 'aux-leaf' }],
})

/**
 * Drives one `ide:save-requested` round trip: the event's own `listen` is spied on so the captured
 * callback can be invoked directly, which is the only way to reach the handler without a backend.
 * `gcTime: Infinity` on the seed because the test client collects observer-less queries immediately
 * (`docs/memory/test-conventions.md` §3).
 */
const requestIdeSave = async (layout: ProjectLayout, path: string) => {
    let deliver: EventCallback<IdeSaveRequested> | null = null
    spyOn(events.ideSaveRequested, 'listen').mockImplementation((handler) => {
        deliver = handler
        return Promise.resolve(() => undefined)
    })
    const resolveSave = spyOn(commands, 'ideResolveSave')

    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => layout, gcTime: Infinity })
    const { IdeSyncProvider } = await import('@app/providers/ide-sync-provider')
    renderWithProviders(<IdeSyncProvider />, { queryClient })
    await act(async () => undefined)

    const payload: IdeSaveRequested = { requestId: REQUEST_ID, projectId: PROJECT_ID, path }
    await act(async () => {
        deliver?.({ event: 'ide:save-requested', id: FAKE_EVENT_ID, payload })
    })
    return resolveSave
}

/**
 * The false "Document saved successfully" of audit #2: `move_tab_to_new_window` removes the tab from
 * the main tree, so a dirty file living in an auxiliary window was not found, `!tab?.dirty` was true,
 * and the agent was told the write had landed while the disk kept the old bytes.
 */
describe('IdeSyncProvider save 요청 대상 탐색', () => {
    beforeAll(() => {
        window.__TAURI_INTERNALS__ = { transformCallback: () => FAKE_EVENT_ID, invoke: fakeInvoke }
        Reflect.set(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY, { unregisterListener: () => undefined })
    })

    afterAll(() => {
        window.__TAURI_INTERNALS__ = undefined
        Reflect.deleteProperty(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY)
    })

    test('보조 창에만 있는 더티 파일에 저장됨(true) 으로 답하지 않는다', async () => {
        const layout = buildLayout([], [buildFileTab('aux-tab', DIRTY_PATH, true)])

        const resolveSave = await requestIdeSave(layout, DIRTY_PATH)

        expect(resolveSave.mock.calls).toEqual([[REQUEST_ID, false]])
    })

    test('어느 창에도 더티 탭이 없으면 종전대로 저장됨(true) 으로 답한다', async () => {
        const layout = buildLayout([buildFileTab('main-tab', CLEAN_PATH, false)], [])

        const resolveSave = await requestIdeSave(layout, CLEAN_PATH)

        expect(resolveSave.mock.calls).toEqual([[REQUEST_ID, true]])
    })
})
