import { beforeEach, describe, expect, mock, test } from 'bun:test'
import * as sonner from 'sonner'
import type { ProjectLayout, SearchQuery, Settings } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * Search as you type (§1.B B4): the panel runs the query while the term is being typed, and Enter
 * keeps every property it had — immediate, history-recording, toast-reporting. What is locked here
 * is the difference between the two triggers, because that is where the regressions live: a
 * debounce that fires per keystroke would hammer the backend, a live run that recorded history
 * would fill the dropdown with prefixes, and a live run that toasted would stack one error
 * notification per character of a regex still being written.
 *
 * Four module fakes, registered before the container is pulled in through a *dynamic* `import()`
 * (`mock.module` is process-global and last-registration-wins — `docs/memory/test-conventions.md`
 * §3): the real `sonner` namespace is snapshotted before registration so no export disappears for
 * whatever file runs next, `@entities/search/search.ipc` is the assertion surface (which query ran,
 * and when), `@entities/settings/settings.ipc` covers both the settings the panel reads and the
 * history write it must not make, and `@shared/lib/monaco/setup` follows the existing precedent —
 * `entities/layout/layout.query` reaches monaco at import time.
 */
const realSonner = { ...sonner }

const errorMessages: string[] = []
const ignoreToast = () => undefined
const toastFake = Object.assign(ignoreToast, {
    ...realSonner.toast,
    error: (message: unknown) => {
        errorMessages.push(String(message))
    },
})

const searchRunCalls: SearchQuery[] = []
const settingsWritePatches: Record<string, unknown>[] = []
const searchRunOutcome = { shouldFail: false }

const rejectLikeUnavailableIpc = () => Promise.reject(new Error('ipc unavailable under bun:test'))

mock.module('sonner', () => ({ ...realSonner, toast: toastFake }))
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

mock.module('@entities/search/search.ipc', () => ({
    runSearch: (input: { query: SearchQuery }) => {
        searchRunCalls.push(input.query)
        return searchRunOutcome.shouldFail ? Promise.reject(new Error(SEARCH_FAILURE_MESSAGE)) : Promise.resolve(0)
    },
    cancelSearch: () => Promise.resolve(),
    replaceSearch: rejectLikeUnavailableIpc,
    listProjectFiles: () => Promise.resolve([]),
}))

mock.module('@entities/settings/settings.ipc', () => ({
    emptySettingsPatch: () => ({}),
    getSettings: () => Promise.resolve(buildSettings()),
    updateSettings: (patch: Record<string, unknown>) => {
        settingsWritePatches.push(patch)
        return Promise.resolve(buildSettings())
    },
    setThemeId: rejectLikeUnavailableIpc,
}))

mock.module('@entities/layout/layout.ipc', () => ({
    getLayout: () => Promise.resolve(null as ProjectLayout | null),
    openTab: rejectLikeUnavailableIpc,
    closeTab: rejectLikeUnavailableIpc,
    activateTab: rejectLikeUnavailableIpc,
    moveTab: rejectLikeUnavailableIpc,
    splitPane: rejectLikeUnavailableIpc,
    openTabInSplit: rejectLikeUnavailableIpc,
    resizePane: rejectLikeUnavailableIpc,
    focusPane: rejectLikeUnavailableIpc,
    pinTab: rejectLikeUnavailableIpc,
    setTabPreview: rejectLikeUnavailableIpc,
    setTabDirty: rejectLikeUnavailableIpc,
    setTerminalSession: rejectLikeUnavailableIpc,
    reopenClosedTab: rejectLikeUnavailableIpc,
    openUntitledTab: rejectLikeUnavailableIpc,
    convertUntitledTab: rejectLikeUnavailableIpc,
    moveTabToWindow: rejectLikeUnavailableIpc,
    setShellView: rejectLikeUnavailableIpc,
    setTabViewState: rejectLikeUnavailableIpc,
    applyTabPathChange: rejectLikeUnavailableIpc,
}))

const SEARCH_FAILURE_MESSAGE = 'regex parse error'
const PROJECT_ID = 'project-1'
const IGNORED_SCOPE_DIR = 'node_modules/pkg'
const DEBOUNCE_MS = 20
const SETTLE_MS = DEBOUNCE_MS * 5

const SETTINGS_VERSION = 1

const buildSettings = (searchOnType = true): Settings => ({
    version: SETTINGS_VERSION,
    searchOnType,
    searchOnTypeDebounceMs: DEBOUNCE_MS,
    recentSearches: [],
})

const importContainer = () => import('@widgets/search-panel/search-panel-container')

const renderPanel = async (searchOnType = true, scopeDir: string | null = null) => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, buildSettings(searchOnType))

    const { SearchPanelContainer } = await importContainer()
    const view = renderWithProviders(
        <TooltipProvider>
            <SearchPanelContainer
                projectId={PROJECT_ID}
                onOpenMatch={() => undefined}
                includeGlob={null}
                scopeDir={scopeDir}
                onClearScope={() => undefined}
                seedText={null}
                openReplace={false}
                openNonce={0}
            />
        </TooltipProvider>,
        { queryClient },
    )

    return { ...view, input: screen.getByPlaceholderText('search.placeholder') }
}

const typeQuery = (input: HTMLElement, value: string) => fireEvent.change(input, { target: { value } })

const settle = () => act(() => new Promise((resolve) => setTimeout(resolve, SETTLE_MS)))

describe('SearchPanelContainer 실시간 검색 (d-58 §1.B B4)', () => {
    beforeEach(() => {
        searchRunCalls.length = 0
        settingsWritePatches.length = 0
        errorMessages.length = 0
        searchRunOutcome.shouldFail = false
    })

    test('설정이 켜져 있으면 입력이 멈춘 뒤에야 검색이 돈다', async () => {
        const { input } = await renderPanel()

        typeQuery(input, 'foo')
        expect(searchRunCalls).toHaveLength(0)

        await settle()
        expect(searchRunCalls).toHaveLength(1)
        expect(searchRunCalls[0]?.text).toBe('foo')
    })

    test('연속 입력은 마지막 한 번만 실행한다', async () => {
        const { input } = await renderPanel()

        typeQuery(input, 'f')
        typeQuery(input, 'fo')
        typeQuery(input, 'foo')

        await settle()
        expect(searchRunCalls).toHaveLength(1)
        expect(searchRunCalls[0]?.text).toBe('foo')
    })

    test('옵션을 바꿔도 같은 디바운스로 재실행한다', async () => {
        const { input } = await renderPanel()

        typeQuery(input, 'foo')
        await settle()

        fireEvent.click(screen.getByLabelText('search.caseSensitive'))
        await settle()
        expect(searchRunCalls).toHaveLength(2)
        expect(searchRunCalls[1]?.caseSensitive).toBe(true)
    })

    test('실시간 실행은 검색 이력을 쌓지 않는다', async () => {
        const { input } = await renderPanel()

        typeQuery(input, 'foo')
        await settle()

        expect(searchRunCalls).toHaveLength(1)
        expect(settingsWritePatches).toHaveLength(0)
    })

    test('Enter 는 즉시 실행하고 대기 중인 디바운스를 취소하며 이력을 남긴다', async () => {
        const { input } = await renderPanel()

        typeQuery(input, 'foo')
        fireEvent.keyDown(input, { key: 'Enter' })
        expect(searchRunCalls).toHaveLength(1)

        await settle()
        expect(searchRunCalls).toHaveLength(1)
        expect(settingsWritePatches).toHaveLength(1)
        expect(settingsWritePatches[0]?.recentSearches).toEqual(['foo'])
    })

    test('실시간 실행의 실패는 토스트 없이 패널 안에서만 알린다', async () => {
        searchRunOutcome.shouldFail = true
        const { input } = await renderPanel()

        typeQuery(input, '(foo')
        await settle()

        expect(searchRunCalls).toHaveLength(1)
        expect(screen.getByText('search.failed')).toBeTruthy()
        expect(errorMessages).toHaveLength(0)
    })

    test('Enter 실행의 실패는 토스트로 알린다', async () => {
        searchRunOutcome.shouldFail = true
        const { input } = await renderPanel(false)

        typeQuery(input, '(foo')
        fireEvent.keyDown(input, { key: 'Enter' })
        await settle()

        expect(errorMessages).toEqual([SEARCH_FAILURE_MESSAGE])
    })

    test('설정이 꺼져 있으면 입력만으로는 돌지 않고 Enter 에서만 돈다', async () => {
        const { input } = await renderPanel(false)

        expect(screen.getByText('search.pressEnterHint')).toBeTruthy()

        typeQuery(input, 'foo')
        await settle()
        expect(searchRunCalls).toHaveLength(0)

        fireEvent.keyDown(input, { key: 'Enter' })
        expect(searchRunCalls).toHaveLength(1)
        await settle()
    })

    test('설정이 켜져 있으면 안내 문구가 실시간 검색 문구로 바뀐다', async () => {
        await renderPanel()

        expect(screen.getByText('search.liveSearchHint')).toBeTruthy()
    })
})

/**
 * "폴더에서 찾기" 는 include glob 이 아니라 `scopeDir` 로 내려간다 — glob 은 walk 가 이미 내보낸
 * 엔트리에만 걸리고 walk 는 `IGNORED_DIR_NAMES` 를 먼저 가지치기해서, `node_modules` 를 지정한 검색이
 * 조용히 0건을 반환했다(audit wave 2 #23).
 */
describe('SearchPanelContainer 폴더 범위 (d-67 #23)', () => {
    beforeEach(() => {
        searchRunCalls.length = 0
        settingsWritePatches.length = 0
        errorMessages.length = 0
        searchRunOutcome.shouldFail = false
    })

    test('폴더 범위는 includeGlob 이 아니라 scopeDir 로 실행된다', async () => {
        const { input } = await renderPanel(false, IGNORED_SCOPE_DIR)

        typeQuery(input, 'version')
        fireEvent.keyDown(input, { key: 'Enter' })

        expect(searchRunCalls).toHaveLength(1)
        expect(searchRunCalls[0]?.scopeDir).toBe(IGNORED_SCOPE_DIR)
        expect(searchRunCalls[0]?.includeGlob).toBeNull()
        await settle()
    })

    test('범위 칩은 그대로 표시된다', async () => {
        await renderPanel(false, IGNORED_SCOPE_DIR)

        expect(screen.getByText('explorer.searchScopeLabel')).toBeTruthy()
    })

    test('범위가 없으면 scopeDir 없이 프로젝트 전체를 검색한다', async () => {
        const { input } = await renderPanel(false)

        typeQuery(input, 'version')
        fireEvent.keyDown(input, { key: 'Enter' })

        expect(searchRunCalls[0]?.scopeDir).toBeNull()
        await settle()
    })
})
