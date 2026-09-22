import { afterEach, describe, expect, mock, spyOn, test } from 'bun:test'
import type { FC } from 'react'
import type { MirrorEntry, ProjectLayout, Tab } from '@shared/api/bindings'
import * as fileIpc from '@entities/file/file.ipc'
import { registerSaveRequest, unregisterSaveRequest } from '@entities/editor/save-request-registry'
import * as layoutIpc from '@entities/layout/layout.ipc'
import { QUERY_KEY } from '@shared/constants/query-key'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'
/** Type-only, so it is erased before runtime and does not pull the hook in ahead of the `mock.module` calls below. */
import type { useRequestCloseTab as UseRequestCloseTab } from '@widgets/editor-area/use-request-close-tab'

/**
 * The unsaved-changes gate in front of every close (audit wave 2 #8). What matters is the *order of
 * IPC*, because that is where the data loss lived: closing publishes `layout_close_tab`, whose
 * success handler clears the file's hot-exit mirror and disposes its monaco buffer, so any save has
 * to have landed first and a failed one must leave the tab open.
 *
 * `@shared/lib/monaco/setup` is stubbed because `entities/editor/model-registry` (and
 * `entities/layout/layout.query` behind it) touch monaco at import time
 * (`docs/memory/test-conventions.md` §3). With no monaco there are no models, so a tab under test
 * takes the *unmounted* branch — the one a background tab in a "Close All" actually takes — and its
 * draft comes from the seeded hot-exit mirror, unless the case registers a save request of its own
 * to stand in for a mounted pane. `@tauri-apps/plugin-dialog` is stubbed for the untitled Save As
 * path; the file cases never reach it.
 *
 * The write itself is spied on `@entities/file/file.ipc`'s namespace rather than on `commands`:
 * three other files register a process-global `mock.module` for that module (§3), so a `commands`
 * spy is simply never reached once one of them has run, and these cases passed alone but not in a
 * full run. The namespace spy sits on whichever implementation is current, mocked or real.
 */
mock.module('@shared/lib/monaco/setup', () => ({
    monaco: { Uri: { file: () => ({ toString: () => '' }), parse: () => ({ toString: () => '' }) }, editor: {} },
}))

const saveDialogPathRef: { current: string | null } = { current: null }

mock.module('@tauri-apps/plugin-dialog', () => ({ save: () => Promise.resolve(saveDialogPathRef.current), open: () => Promise.resolve(null) }))

const importHook = () => import('@widgets/editor-area/use-request-close-tab')

const PROJECT_ID = 'project-1'
const DIRTY_PATH = '/repo/dirty.ts'
const OTHER_DIRTY_PATH = '/repo/other.ts'
const DIRTY_CONTENT = 'const answer = 42\n'
const OTHER_DIRTY_CONTENT = 'export const other = true\n'
const WRITE_FAILED = new Error('disk is full')

const fileTab = (id: string, path: string, dirty: boolean): Tab => ({ id, kind: { kind: 'file', path }, title: id, dirty })
const terminalTab = (id: string): Tab => ({ id, kind: { kind: 'terminal', sessionId: id }, title: id, dirty: true })

const MIRRORS: MirrorEntry[] = [
    { path: DIRTY_PATH, content: DIRTY_CONTENT, savedAtMs: null, diskModifiedMs: null, conflict: false, sourceMissing: false },
    { path: OTHER_DIRTY_PATH, content: OTHER_DIRTY_CONTENT, savedAtMs: null, diskModifiedMs: null, conflict: false, sourceMissing: false },
]

const LAYOUT: ProjectLayout = { version: 2, root: { node: 'leaf', id: 'leaf-1', tabs: [], active: null }, focusedPane: 'leaf-1', revision: 1 }

/** Drains the mutation's own promise chain (retryer hop, `onSettled`, the notify-manager microtask) so its React Query state lands inside the `act` scope the click was fired in. */
const settleMutations = async () => {
    for (let hop = 0; hop < 6; hop += 1) await new Promise((resolve) => setTimeout(resolve, 0))
}

type HarnessProps = { tabs: Tab[]; useHook: typeof UseRequestCloseTab }

const Harness: FC<HarnessProps> = ({ tabs, useHook }) => {
    const { requestCloseTabs, closeDirtyTabDialog } = useHook(PROJECT_ID)

    return (
        <>
            <button type='button' onClick={() => requestCloseTabs(tabs)}>
                request close
            </button>
            {closeDirtyTabDialog}
        </>
    )
}

/** `gcTime: Infinity` on every seed because the test client collects observer-less queries immediately (`docs/memory/test-conventions.md` §3). */
const renderHarness = async (tabs: Tab[]) => {
    const queryClient = createTestQueryClient()
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.LAYOUT.DETAIL(PROJECT_ID), queryFn: () => LAYOUT, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.FILE.MIRRORS(PROJECT_ID), queryFn: () => MIRRORS, gcTime: Infinity })
    await queryClient.fetchQuery({ queryKey: QUERY_KEY.FILE.UNTITLED_MIRRORS(PROJECT_ID), queryFn: () => [], gcTime: Infinity })

    const { useRequestCloseTab } = await importHook()
    const rendered = renderWithProviders(<Harness tabs={tabs} useHook={useRequestCloseTab} />, { queryClient })
    await act(async () => {
        fireEvent.click(screen.getByRole('button', { name: 'request close' }))
        await settleMutations()
    })
    return rendered
}

const answer = async (key: string) => {
    await act(async () => {
        fireEvent.click(screen.getByRole('button', { name: key }))
        await settleMutations()
    })
}

const spyOnCloseTab = () => spyOn(layoutIpc, 'closeTab').mockResolvedValue(LAYOUT)
const spyOnSetDirty = () => spyOn(layoutIpc, 'setTabDirty').mockResolvedValue(LAYOUT)
const dialogTitle = () => screen.queryByText('tab.confirmCloseDirtyTitle')

describe('useRequestCloseTab 더티 게이트', () => {
    afterEach(() => unregisterSaveRequest('dirty'))

    test('저장된 탭은 묻지 않고 바로 닫는다', async () => {
        const closeTab = spyOnCloseTab()

        await renderHarness([fileTab('clean', DIRTY_PATH, false)])

        expect(closeTab.mock.calls.map(([tabId]) => tabId)).toEqual(['clean'])
        expect(dialogTitle()).toBeNull()
    })

    test('텍스트를 들고 있지 않은 탭은 dirty 여도 즉시 닫는다 — 게이트는 file·untitled 전용', async () => {
        const closeTab = spyOnCloseTab()

        await renderHarness([terminalTab('term-a')])

        expect(closeTab.mock.calls.map(([tabId]) => tabId)).toEqual(['term-a'])
        expect(dialogTitle()).toBeNull()
    })

    test('dirty 파일 탭은 먼저 묻고, 대답 전에는 닫지 않는다', async () => {
        const closeTab = spyOnCloseTab()

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])

        expect(dialogTitle()).toBeTruthy()
        expect(closeTab).not.toHaveBeenCalled()
    })

    test('취소는 아무 것도 하지 않는다', async () => {
        const closeTab = spyOnCloseTab()
        const setDirty = spyOnSetDirty()

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('common.cancel')

        expect(closeTab).not.toHaveBeenCalled()
        expect(setDirty).not.toHaveBeenCalled()
        expect(dialogTitle()).toBeNull()
    })

    test('저장 안 함은 dirty 를 먼저 끈 뒤 닫는다 — 닫힌 탭 스택에 유령 dirty 가 남지 않게', async () => {
        const order: string[] = []
        spyOnSetDirty().mockImplementation(async ({ tabId, dirty }) => {
            order.push(`set-dirty:${tabId}:${dirty}`)
            return LAYOUT
        })
        spyOnCloseTab().mockImplementation(async (tabId) => {
            order.push(`close:${tabId}`)
            return LAYOUT
        })
        const saveFile = spyOn(fileIpc, 'saveFile').mockResolvedValue(null)

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('tab.confirmCloseDirtyDiscard')

        expect(order).toEqual(['set-dirty:dirty:false', 'close:dirty'])
        expect(saveFile).not.toHaveBeenCalled()
    })

    test('저장은 미러에 남은 초안을 디스크에 쓴 뒤 닫는다 — 언마운트된 탭도 같은 경로', async () => {
        const order: string[] = []
        const saveFile = spyOn(fileIpc, 'saveFile').mockImplementation(async ({ path, content }) => {
            order.push(`save:${path}:${content}`)
            return null
        })
        spyOnCloseTab().mockImplementation(async (tabId) => {
            order.push(`close:${tabId}`)
            return LAYOUT
        })

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('tab.confirmCloseDirtySave')

        expect(saveFile).toHaveBeenCalledTimes(1)
        expect(order).toEqual([`save:${DIRTY_PATH}:${DIRTY_CONTENT}`, 'close:dirty'])
    })

    test('저장이 실패하면 닫지 않는다 — 닫기가 미러까지 지우기 때문', async () => {
        spyOn(fileIpc, 'saveFile').mockRejectedValue(WRITE_FAILED)
        const closeTab = spyOnCloseTab()

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('tab.confirmCloseDirtySave')

        expect(closeTab).not.toHaveBeenCalled()
    })

    test('여러 탭을 닫아도 질문은 한 번이고, 그 답이 요청 전체에 적용된다', async () => {
        const setDirty = spyOnSetDirty()
        const closeTab = spyOnCloseTab()
        const tabs = [fileTab('dirty-a', DIRTY_PATH, true), fileTab('clean', '/repo/clean.ts', false), fileTab('dirty-b', OTHER_DIRTY_PATH, true)]

        await renderHarness(tabs)

        expect(screen.getAllByText('tab.confirmCloseDirtyTitle')).toHaveLength(1)

        await answer('tab.confirmCloseDirtyDiscard')

        expect(setDirty.mock.calls.map(([input]) => input)).toEqual([
            { tabId: 'dirty-a', dirty: false },
            { tabId: 'dirty-b', dirty: false },
        ])
        expect(closeTab.mock.calls.map(([tabId]) => tabId)).toEqual(['dirty-a', 'clean', 'dirty-b'])
    })

    test('확인이 떠 있는 동안에는 다른 진입점의 닫기 요청을 무시한다 — ⌘W 가 질문 위에 질문을 쌓지 않는다', async () => {
        const closeTab = spyOnCloseTab()
        const queryClient = createTestQueryClient()
        await queryClient.fetchQuery({ queryKey: QUERY_KEY.FILE.MIRRORS(PROJECT_ID), queryFn: () => MIRRORS, gcTime: Infinity })
        const { useRequestCloseTab } = await importHook()

        renderWithProviders(
            <>
                <Harness tabs={[fileTab('dirty', DIRTY_PATH, true)]} useHook={useRequestCloseTab} />
                <Harness tabs={[fileTab('clean', '/repo/clean.ts', false)]} useHook={useRequestCloseTab} />
            </>,
            { queryClient },
        )

        const [askingEntry, otherEntry] = screen.getAllByRole('button', { name: 'request close' })
        await act(async () => {
            fireEvent.click(askingEntry)
            await settleMutations()
        })
        await act(async () => {
            fireEvent.click(otherEntry)
            await settleMutations()
        })

        expect(screen.getAllByText('tab.confirmCloseDirtyTitle')).toHaveLength(1)
        expect(closeTab).not.toHaveBeenCalled()
    })

    test('마운트된 탭의 저장은 pane 저장 파이프라인이 끝나기 전에는 닫지 않는다', async () => {
        const closeTab = spyOnCloseTab()
        const write: { release: ((saved: boolean) => void) | null } = { release: null }
        registerSaveRequest(
            'dirty',
            () =>
                new Promise<boolean>((resolve) => {
                    write.release = resolve
                }),
        )

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('tab.confirmCloseDirtySave')

        expect(closeTab).not.toHaveBeenCalled()

        await act(async () => {
            write.release?.(true)
            await settleMutations()
        })

        expect(closeTab.mock.calls.map(([tabId]) => tabId)).toEqual(['dirty'])
    })

    test('마운트된 탭의 pane 저장이 실패를 보고하면 닫지 않고, 초안을 뒤로 돌려 쓰지도 않는다', async () => {
        const closeTab = spyOnCloseTab()
        const saveFile = spyOn(fileIpc, 'saveFile').mockResolvedValue(null)
        registerSaveRequest('dirty', () => Promise.resolve(false))

        await renderHarness([fileTab('dirty', DIRTY_PATH, true)])
        await answer('tab.confirmCloseDirtySave')

        expect(closeTab).not.toHaveBeenCalled()
        expect(saveFile).not.toHaveBeenCalled()
    })

    test('untitled 탭의 저장은 Save As 를 거치고, 사용자가 위치 선택을 취소하면 닫지 않는다', async () => {
        saveDialogPathRef.current = null
        const saveFile = spyOn(fileIpc, 'saveFile').mockResolvedValue(null)
        const closeTab = spyOnCloseTab()

        await renderHarness([{ id: 'untitled-1', kind: { kind: 'untitled', index: 1 }, title: 'Untitled-1', dirty: true }])
        await answer('tab.confirmCloseDirtySave')

        expect(saveFile).not.toHaveBeenCalled()
        expect(closeTab).not.toHaveBeenCalled()
    })
})
