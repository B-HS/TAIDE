import { afterAll, afterEach, beforeAll, describe, expect, mock, spyOn, test } from 'bun:test'
import type { FC } from 'react'
import { useEffect, useRef } from 'react'
import type {
    AppError,
    PaneNode,
    ProjectLayout,
    PtySpawnOptions,
    ResolvedTheme,
    Settings,
    TerminalSession as TerminalSessionInfo,
} from '@shared/api/bindings'
import { commands } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { DEFAULT_SCROLLBACK_BYTES, MAX_SCROLLBACK_BYTES, SCROLLBACK_BYTES_PER_LINE_ESTIMATE } from '@shared/constants/terminal'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

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

type TerminalPaneStubProps = {
    sessionId: string | null
    onReady: (cols: number, rows: number) => void
    attachData: (onData: (bytes: Uint8Array, backlogBytes: number) => void) => () => void
}

const TerminalPaneStub: FC<TerminalPaneStubProps> = ({ sessionId, onReady, attachData }) => {
    const hasMeasuredRef = useRef(false)
    const attachDataRef = useRef(attachData)

    /** xterm measures once; the ref keeps this to one call even though `onReady` is a fresh closure on every render of the widget above. */
    useEffect(() => {
        if (hasMeasuredRef.current) return
        hasMeasuredRef.current = true
        onReady(READY_COLS, READY_ROWS)
    }, [onReady])

    /**
     * Stands in for the real `TerminalPane`'s subscribe effect, which is the only thing that ever
     * issues `pty_attach` — without it a re-attach is indistinguishable from doing nothing at all.
     * Mirrors that component's ref-per-render shape so the subscription is keyed on the session alone
     * and not re-armed by every fresh `attachData` closure.
     */
    useEffect(() => {
        attachDataRef.current = attachData
    })

    useEffect(() => {
        if (!sessionId) return
        return attachDataRef.current(() => undefined)
    }, [sessionId])

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
const PTY_COMMAND_PREFIX = 'pty_'
const SPAWN_COMMAND = 'pty_spawn'
const ATTACH_COMMAND = 'pty_attach'
const IPC_UNAVAILABLE_ERROR = { code: 'Internal', message: 'no backend in the test harness' } satisfies AppError

/** `pty_spawn` goes through raw `invoke` (it carries a `Channel`), so a test that needs the spawn to *land* says so here rather than through `commands` — the mutable-reference shape `docs/memory/test-conventions.md` §3 prescribes for a per-test fake. */
const spawnedSessionIdRef: { current: string | null } = { current: null }

/** Every `pty_*` invocation in arrival order, so a test can assert which of spawn/attach a mount chose and with what options. */
const ptyInvocations: { command: string; args: unknown }[] = []

/**
 * Event subscription has to succeed (the widget registers two listeners during mount, and `listen()`
 * reaches `transformCallback` synchronously), while every real command has to fail the way it does
 * everywhere else in the harness — resolving them all would feed `layout_get` a number and overwrite
 * the seeded layout with it.
 */
const fakeInvoke = (command: string, args?: unknown) => {
    if (command.startsWith(EVENT_PLUGIN_COMMAND_PREFIX)) return Promise.resolve(FAKE_EVENT_ID)
    if (command.startsWith(PTY_COMMAND_PREFIX)) ptyInvocations.push({ command, args })
    if (command === SPAWN_COMMAND && spawnedSessionIdRef.current) return Promise.resolve(spawnedSessionIdRef.current)
    return Promise.reject(IPC_UNAVAILABLE_ERROR)
}

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

const SETTINGS_VERSION = 1

const buildSettings = (terminalScrollback?: number): Settings => ({ version: SETTINGS_VERSION, terminalScrollback })

const buildLiveSession = (id: string): TerminalSessionInfo => ({ id, projectId: PROJECT_ID, cwd: MAIN_CWD, shell: 'default', running: true })

type RenderOverrides = { sessionId?: string; liveSessions?: TerminalSessionInfo[]; settings?: Settings }

/** `gcTime: Infinity` on every seed because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderTerminalSession = async (tabId: string, paneId: string, { sessionId = '', liveSessions = [], settings }: RenderOverrides = {}) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => LAYOUT, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.THEME.CURRENT, queryFn: () => THEME, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.TERMINAL.SESSIONS(PROJECT_ID), queryFn: () => liveSessions, gcTime: Infinity })
    if (settings) await queryClient.fetchQuery({ queryKey: QUERY_KEY.SETTINGS.CURRENT, queryFn: () => settings, gcTime: Infinity })

    const { TerminalSession } = await importTerminalSession()
    const rendered = renderWithProviders(
        <TerminalSession
            projectId={PROJECT_ID}
            tabId={tabId}
            paneId={paneId}
            sessionId={sessionId}
            autoFocus={false}
            paneElementRef={{ current: null }}
        />,
        { queryClient },
    )
    await act(async () => undefined)
    return rendered
}

const ptyCommandsIssued = () => ptyInvocations.map((invocation) => invocation.command)

const spawnedOptions = () => (ptyInvocations.find((invocation) => invocation.command === SPAWN_COMMAND)?.args as { opts?: PtySpawnOptions })?.opts

const TAURI_EVENT_PLUGIN_INTERNALS_KEY = '__TAURI_EVENT_PLUGIN_INTERNALS__'

/**
 * Installed for the whole file rather than per test: the harness's own `afterEach(cleanup)` unmounts
 * the tree, and the unmount calls `unlisten`, which reaches the event plugin's globals — clearing
 * them between tests would move the crash into cleanup instead.
 */
beforeAll(() => {
    window.__TAURI_INTERNALS__ = { transformCallback: () => FAKE_EVENT_ID, invoke: fakeInvoke }
    Reflect.set(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY, { unregisterListener: () => undefined })
})

afterAll(() => {
    window.__TAURI_INTERNALS__ = undefined
    Reflect.deleteProperty(window, TAURI_EVENT_PLUGIN_INTERNALS_KEY)
})

afterEach(() => {
    enterWindowUrl(MAIN_WINDOW_URL)
    spawnedSessionIdRef.current = null
    ptyInvocations.length = 0
})

describe('TerminalSession 창별 cwd', () => {
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

/**
 * A spawn failure is usually a setting the user can put back (`pty_default_options` forwards the
 * unvalidated `shell_override` free text), but the failure branch used to render a bare message and
 * return before `TerminalPane` — so the `spawnStartedRef` reset `handleSpawnFailure` performs for a
 * retry could never be read again, and the tab stayed dead for its whole lifetime.
 */
describe('TerminalSession 스폰 실패 복구', () => {
    test('스폰이 실패하면 재시작 버튼을 보여준다', async () => {
        spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'error', error: IPC_UNAVAILABLE_ERROR })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf')

        expect(screen.getByRole('button', { name: 'terminal.restart' })).toBeTruthy()
    })

    test('재시작 버튼이 다시 스폰하고, 성공하면 에러 화면에서 빠져나온다', async () => {
        const ptyDefaultOptions = spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'error', error: IPC_UNAVAILABLE_ERROR })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf')

        ptyDefaultOptions.mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })
        spawnedSessionIdRef.current = 'session-restarted'
        await act(async () => {
            fireEvent.click(screen.getByRole('button', { name: 'terminal.restart' }))
        })

        expect(ptyDefaultOptions.mock.calls).toEqual([
            [PROJECT_ID, MAIN_CWD],
            [PROJECT_ID, MAIN_CWD],
        ])
        expect(screen.queryByRole('button', { name: 'terminal.restart' })).toBeNull()
    })
})

/**
 * d-67 #6 — the terminal roster is a per-window `staleTime: Infinity` cache, so a session spawned in
 * one window was invisible to the window a terminal tab was then dragged into: that window read the
 * still-running session as dead and spawned a replacement over it, orphaning the original pty.
 * `ipc-sync-provider.tsx` now converges every window's roster from the `terminal:spawned` event; what
 * this pins is the consumer half — a roster that *does* know the session must re-attach, never spawn.
 */
describe('TerminalSession 재attach 판정', () => {
    test('로스터가 살아 있다고 말하는 세션은 새로 스폰하지 않고 attach 한다', async () => {
        const liveSessionId = 'session-from-other-window'

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf', { sessionId: liveSessionId, liveSessions: [buildLiveSession(liveSessionId)] })

        expect(ptyCommandsIssued()).toContain(ATTACH_COMMAND)
        expect(ptyCommandsIssued()).not.toContain(SPAWN_COMMAND)
    })

    test('로스터에 없는 세션 id 로 마운트하면 종전대로 새로 스폰한다', async () => {
        spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf', { sessionId: 'session-unknown', liveSessions: [] })

        expect(ptyCommandsIssued()).toContain(SPAWN_COMMAND)
    })
})

/**
 * d-67 #18 — the scrollback setting only ever resized xterm's display buffer while the Rust ring that
 * every tab switch replays from stayed at a fixed 2 MiB, so raising it changed nothing the user could
 * see and lowering it changed nothing either. The spawn now carries the converted byte budget.
 */
describe('TerminalSession 스크롤백 예산', () => {
    test('설정 줄 수를 바이트 예산으로 환산해 스폰 옵션에 싣는다', async () => {
        const scrollbackLines = 20_000
        spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf', { settings: buildSettings(scrollbackLines) })

        expect(spawnedOptions()?.scrollbackBytes).toBe(scrollbackLines * SCROLLBACK_BYTES_PER_LINE_ESTIMATE)
    })

    test('설정 최댓값은 상한으로 잘려 나간다', async () => {
        spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf', { settings: buildSettings(100_000) })

        expect(spawnedOptions()?.scrollbackBytes).toBe(MAX_SCROLLBACK_BYTES)
    })

    test('설정이 없으면 예전과 같은 기본 예산으로 스폰한다', async () => {
        spyOn(commands, 'ptyDefaultOptions').mockResolvedValue({ status: 'ok', data: spawnOptionsFor(MAIN_CWD) })

        await renderTerminalSession(MAIN_TAB_ID, 'main-leaf', { settings: buildSettings() })

        expect(spawnedOptions()?.scrollbackBytes).toBe(DEFAULT_SCROLLBACK_BYTES)
    })
})
