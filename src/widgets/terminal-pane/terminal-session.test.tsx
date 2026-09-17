import { afterAll, afterEach, beforeAll, describe, expect, mock, spyOn, test } from 'bun:test'
import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import type { PaneNode, ProjectLayout, PtySpawnOptions, ResolvedTheme } from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, renderWithProviders } from '@shared/testing/render'

/**
 * A terminal opened from an auxiliary window ("Open in Terminal" on a folder in that window's
 * explorer) only ever exists in that window's `AuxWindowLayout`, so looking its tab up in
 * `layout.root` found nothing, `tabCwd` fell to `null`, and `pty_default_options` spawned the shell
 * at the project root with no error to show for it (audit #4). What the second argument of
 * `pty_default_options` is, is the whole observable — nothing else reads the tab's `cwd`.
 *
 * Two module fakes, registered before the widget is pulled in through a *dynamic* `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3): `@shared/lib/monaco/setup`, reached at import time through `entities/layout/layout.query`, and
 * `TerminalPane`, which mounts xterm against a canvas the harness does not have (§5). The stub
 * stands in for xterm's measuring pass by calling `onReady` once, which is what starts the spawn.
 *
 * `window.__TAURI_INTERNALS__` is seeded and removed per test the way
 * `auxiliary-window-shell.test.tsx` does it (§4): this widget subscribes to two Tauri events during
 * mount, and `listen()` reaches `transformCallback` *synchronously*, so without the shim the mount
 * throws before any pty call is made.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const READY_COLS = 80
const READY_ROWS = 24

type TerminalPaneStubProps = { onReady: (cols: number, rows: number) => void }

const TerminalPaneStub: FC<TerminalPaneStubProps> = ({ onReady }) => {
    const hasMeasuredRef = useRef(false)

    /** xterm measures once; the ref keeps this to one call even though `onReady` is a fresh closure on every render of the widget above. */
    useEffect(() => {
        if (hasMeasuredRef.current) return
        hasMeasuredRef.current = true
        onReady(READY_COLS, READY_ROWS)
    }, [onReady])

    return <div />
}

mock.module('@widgets/terminal-pane/terminal-pane', () => ({ TerminalPane: TerminalPaneStub }))

const importTerminalSession = () => import('@widgets/terminal-pane/terminal-session')

const PROJECT_ID = 'project-1'
const MAIN_TAB_ID = 'main-terminal'
const AUXILIARY_TAB_ID = 'aux-terminal'
const MAIN_CWD = '/repo'
const AUXILIARY_CWD = '/repo/src/widgets'
const FAKE_EVENT_ID = 1
const EVENT_PLUGIN_COMMAND_PREFIX = 'plugin:event'
const IPC_UNAVAILABLE_ERROR = { code: 'Unknown', message: 'no backend in the test harness' }

/**
 * Event subscription has to succeed (the widget registers two listeners during mount, and `listen()`
 * reaches `transformCallback` synchronously), while every real command has to fail the way it does
 * everywhere else in the harness — resolving them all would feed `layout_get` a number and overwrite
 * the seeded layout with it.
 */
const fakeInvoke = (command: string) =>
    command.startsWith(EVENT_PLUGIN_COMMAND_PREFIX) ? Promise.resolve(FAKE_EVENT_ID) : Promise.reject(IPC_UNAVAILABLE_ERROR)

const MAIN_WINDOW_URL = '/'
const AUXILIARY_WINDOW_URL = `/?projectId=${PROJECT_ID}&windowSlot=1`

/** See `pane-tree.test.ts` — the harness pins `location.search` to empty, and this is the one same-document way to move it. */
const enterWindowUrl = (url: string) => window.history.replaceState({}, '', url)

const THEME: ResolvedTheme = { id: 'test', name: 'Test', type: 'dark', colors: {}, syntax: {}, terminal: {} }

const buildTerminalLeaf = (paneId: string, tabId: string, cwd: string): PaneNode => ({
    node: 'leaf',
    id: paneId,
    tabs: [{ id: tabId, kind: { kind: 'terminal', sessionId: '', cwd }, title: 'terminal' }],
    active: tabId,
})

const LAYOUT: ProjectLayout = {
    version: 2,
    root: buildTerminalLeaf('main-leaf', MAIN_TAB_ID, MAIN_CWD),
    focusedPane: 'main-leaf',
    auxiliaryWindows: [{ slot: 1, root: buildTerminalLeaf('aux-leaf', AUXILIARY_TAB_ID, AUXILIARY_CWD), focusedPane: 'aux-leaf' }],
}

const spawnOptionsFor = (cwd: string): PtySpawnOptions => ({ projectId: PROJECT_ID, cwd, shell: null, cols: READY_COLS, rows: READY_ROWS })

/** `gcTime: Infinity` on every seed because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderTerminalSession = async (tabId: string, paneId: string) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => LAYOUT, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.THEME.CURRENT, queryFn: () => THEME, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.TERMINAL.SESSIONS(PROJECT_ID), queryFn: () => [], gcTime: Infinity })

    const { TerminalSession } = await importTerminalSession()
    const rendered = renderWithProviders(
        <TerminalSession projectId={PROJECT_ID} tabId={tabId} paneId={paneId} sessionId='' autoFocus={false} paneElementRef={{ current: null }} />,
        { queryClient },
    )
    await act(async () => undefined)
    return rendered
}

const TAURI_EVENT_PLUGIN_INTERNALS_KEY = '__TAURI_EVENT_PLUGIN_INTERNALS__'

describe('TerminalSession 창별 cwd', () => {
    /**
     * Installed for the whole file rather than per test: the harness's own `afterEach(cleanup)`
     * unmounts the tree, and the unmount calls `unlisten`, which reaches the event plugin's globals —
     * clearing them between tests would move the crash into cleanup instead.
     */
    beforeAll(() => {
        window.__TAURI_INTERNALS__ = { transformCallback: () => FAKE_EVENT_ID, invoke: fakeInvoke }
        Reflect.set(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY, { unregisterListener: () => undefined })
    })

    afterAll(() => {
        window.__TAURI_INTERNALS__ = undefined
        Reflect.deleteProperty(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY)
    })

    afterEach(() => enterWindowUrl(MAIN_WINDOW_URL))

    test('보조 창의 터미널 탭은 그 탭에 저장된 cwd 로 스폰한다', async () => {
        enterWindowUrl(AUXILIARY_WINDOW_URL)
        const ptyDefaultOptions = spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(AUXILIARY_CWD) })

        await renderTerminalSession(AUXILIARY_TAB_ID, 'aux-leaf')

        expect(ptyDefaultOptions.mock.calls).toEqual([[PROJECT_ID, AUXILIARY_CWD]])
    })

    test('main 창의 터미널 탭은 종전대로 main 트리의 cwd 로 스폰한다', async () => {
        const ptyDefaultOptions = spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf')

        expect(ptyDefaultOptions.mock.calls).toEqual([[PROJECT_ID, MAIN_CWD]])
    })
})
