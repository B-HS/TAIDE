import { describe, expect, mock, test } from 'bun:test'
import type { ComponentProps } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import type { GitStashEntry, LogEntry, Settings, SettingsPatch } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, createTestQueryClient, fireEvent, renderWithProviders, screen } from '@shared/testing/render'

/**
 * The SCM panel's two-pane body (d-58 §1.H): the commit graph moved out of the one scrolled list
 * into its own collapsible `Panel`, and its collapse state moved out of module memory into
 * `Settings`. What this locks is the wiring those two moves created — what the header renders from,
 * what a toggle writes, and that ↑↓ still crosses the separator.
 *
 * Neither list's rows can be asserted on: happy-dom reports every element as 0px, and
 * `@tanstack/virtual-core` returns an empty range for a zero-height viewport, so a virtualized row
 * never mounts here (`docs/memory/test-conventions.md` §5). A pane's *body* is therefore identified
 * by the `ScrollContainer` it owns — its `overlay-scrollbar-track` slot — and the panes themselves
 * by the `data-testid` `react-resizable-panels` puts on every `Panel`.
 *
 * What a toggle writes is read back from the `MutationCache` rather than from a stubbed
 * `settings.ipc`: that module is already `mock.module`-ed by two other test files, and those
 * registrations are process-global and last-one-wins (`docs/memory/test-conventions.md` §3), so any
 * stub of the transport here would either be overwritten by them or overwrite theirs. The variables
 * a mutation was started with are recorded whether or not its `mutationFn` then fails, which is
 * exactly what an IPC-less harness needs.
 *
 * The panel reaches monaco through `commit-detail-panel` → `layout.query` → `tab-path-change`, so
 * that module is stubbed before the panel is pulled in through a *dynamic* `import()` (same shape as
 * `entities/layout/use-open-file-tab.test.tsx`). Labels are locale keys because the test i18n
 * instance carries no bundles.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const { GitPanel } = await import('@widgets/git-panel/git-panel')

const PROJECT_ID = 'project-1'
const SCROLL_TRACK_SELECTOR = '[data-slot="overlay-scrollbar-track"]'

const COMMITS: LogEntry[] = [{ id: 'c1', parents: [], summary: 'feat: first commit', author: 'author', timeUnix: 0, refs: [] }]
const STASHES: GitStashEntry[] = [{ index: 0, message: 'stash@{0}' }]

/** The panel's header bar also carries a `git.stashPush` button, so a section header is the match that is one. */
const sectionHeader = (title: RegExp) => {
    const header = screen.getAllByRole('button', { name: title }).find((element) => element.hasAttribute('data-git-section-header'))
    if (!header) throw new Error(`no section header matching ${title}`)
    return header
}

const graphHeader = () => sectionHeader(/^git\.graph/)
const stashHeader = () => sectionHeader(/^git\.stash/)
const graphPane = () => screen.getByTestId('git-graph')
const graphBody = () => graphPane().querySelector(SCROLL_TRACK_SELECTOR)

/**
 * `null` is the cold start: the panel mounts while the settings query has nothing yet. Fetching is
 * disabled for that case rather than left to fail, because whether `settings.ipc` answers at all
 * depends on which other file's `mock.module` won the process (`docs/memory/test-conventions.md` §3)
 * — the cache is then filled explicitly with {@link deliverSettings}.
 */
const renderPanel = (settings: Partial<Settings> | null, props: Partial<ComponentProps<typeof GitPanel>> = {}) => {
    const queryClient = createTestQueryClient()
    if (settings) queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
    else queryClient.setQueryDefaults(QUERY_KEY.SETTINGS.CURRENT, { enabled: false })
    queryClient.setQueryData(QUERY_KEY.GIT.TAGS(PROJECT_ID), [])

    return renderWithProviders(
        <TooltipProvider>
            <GitPanel
                projectId={PROJECT_ID}
                branch='main'
                ahead={0}
                behind={0}
                hasRemote={false}
                remote={null}
                rows={[]}
                commitMessage=''
                onCommitMessageChange={() => {}}
                onCommit={() => {}}
                isCommitting={false}
                onGenerateCommitMessage={() => {}}
                isGeneratingCommitMessage={false}
                onStage={() => {}}
                onUnstage={() => {}}
                onDiscard={() => {}}
                onOpenFile={() => {}}
                onOpenChanges={() => {}}
                onCopyPath={() => {}}
                onRevealInExplorer={() => {}}
                onSync={() => {}}
                isSyncing={false}
                branches={[]}
                stashes={[]}
                canStash={false}
                isStashing={false}
                onStashPush={() => {}}
                onStashApply={() => {}}
                onStashDrop={() => {}}
                onCheckoutBranch={() => {}}
                onCheckoutRemoteBranch={() => {}}
                onCreateBranch={() => {}}
                graphCommits={COMMITS}
                {...props}
            />
        </TooltipProvider>,
        { queryClient },
    )
}

const isSettingsPatch = (value: unknown): value is SettingsPatch => typeof value === 'object' && value !== null && 'gitSectionsCollapsed' in value

/** Every settings patch the panel has sent, in the order it sent them. */
const settingsPatchesOf = (queryClient: QueryClient) =>
    queryClient
        .getMutationCache()
        .getAll()
        .map((mutation) => mutation.state.variables)
        .filter(isSettingsPatch)

/**
 * A settings mutation settles a few microtasks after the click and updates the panel again when it
 * does, so both passes have to happen inside `act` — the second, empty one is what absorbs that
 * trailing render.
 */
const clickAndSettle = async (element: HTMLElement) => {
    await act(async () => {
        fireEvent.click(element)
    })
    await act(async () => {})
}

/** Fills the settings cache after the panel has already mounted, then lets the adopt effect's own render land. */
const deliverSettings = async (queryClient: QueryClient, settings: Partial<Settings>) => {
    await act(async () => {
        queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, settings)
    })
    await act(async () => {})
}

describe('GitPanel 그래프 pane', () => {
    test('그래프는 변경 목록과 다른 pane 에 있다', () => {
        renderPanel({ gitSectionsCollapsed: [] })

        expect(graphPane().contains(graphHeader())).toBe(true)
        expect(screen.getByTestId('git-changes').contains(graphHeader())).toBe(false)
    })

    test('설정이 그래프를 접었다고 하면 헤더만 남고 본문을 그리지 않는다', () => {
        renderPanel({ gitSectionsCollapsed: ['graph'] })

        expect(graphHeader().getAttribute('aria-expanded')).toBe('false')
        expect(graphBody()).toBeNull()
    })

    test('펼쳐져 있으면 그래프 본문이 자기 스크롤 영역으로 들어간다', () => {
        renderPanel({ gitSectionsCollapsed: [] })

        expect(graphHeader().getAttribute('aria-expanded')).toBe('true')
        expect(graphBody()).not.toBeNull()
    })

    test('설정이 아직 없으면 기본값으로 그린다 — 그래프는 펼침', () => {
        renderPanel({})

        expect(graphHeader().getAttribute('aria-expanded')).toBe('true')
    })

    test('커밋이 없으면 그래프 pane 자체를 만들지 않는다', () => {
        renderPanel({ gitSectionsCollapsed: [] }, { graphCommits: [] })

        expect(screen.queryByTestId('git-graph')).toBeNull()
        expect(screen.getByTestId('git-changes')).toBeDefined()
    })

    test('헤더를 클릭하면 접힌 섹션 목록을 설정에 저장한다', async () => {
        const { queryClient } = renderPanel({ gitSectionsCollapsed: [] })

        await clickAndSettle(graphHeader())
        const patches = settingsPatchesOf(queryClient)

        expect(patches.length).toBe(1)
        expect(patches[0].gitSectionsCollapsed).toEqual(['graph'])
    })

    test('펼치기도 같은 경로로 저장한다 — 목록에서 graph 만 빠진다', async () => {
        const { queryClient } = renderPanel({ gitSectionsCollapsed: ['graph', 'stashes'] })

        await clickAndSettle(graphHeader())

        expect(settingsPatchesOf(queryClient)[0].gitSectionsCollapsed).toEqual(['stashes'])
    })

    test('스태시 헤더도 같은 설정 필드에 저장한다', async () => {
        const { queryClient } = renderPanel({ gitSectionsCollapsed: [] }, { stashes: STASHES, canStash: true })

        await clickAndSettle(stashHeader())

        expect(settingsPatchesOf(queryClient)[0].gitSectionsCollapsed).toEqual(['stashes'])
    })
})

/**
 * The panel owns the collapse map and mirrors it into `Settings` (d-58 review G-2). Reading it back
 * from the settings cache meant a toggle only saw the sections a *completed* `settings_update` had
 * reported, which is what these tests reproduce: no IPC answers in this harness, so the cache never
 * moves and the old wiring would drop everything but the most recent toggle.
 */
describe('GitPanel 접힘 상태 정본', () => {
    test('서로 다른 섹션을 연달아 토글하면 마지막 patch 에 두 섹션이 모두 실린다', async () => {
        const { queryClient } = renderPanel({ gitSectionsCollapsed: [] }, { stashes: STASHES, canStash: true })

        await clickAndSettle(graphHeader())
        await clickAndSettle(stashHeader())
        const patches = settingsPatchesOf(queryClient)

        expect(patches.length).toBe(2)
        expect(patches[1].gitSectionsCollapsed).toEqual(['stashes', 'graph'])
    })

    test('토글 전이면 늦게 도착한 설정을 반영한다', async () => {
        const { queryClient } = renderPanel(null)
        expect(graphHeader().getAttribute('aria-expanded')).toBe('true')

        await deliverSettings(queryClient, { gitSectionsCollapsed: ['graph'] })

        expect(graphHeader().getAttribute('aria-expanded')).toBe('false')
    })

    test('사용자가 토글한 뒤 도착한 설정은 로컬 상태를 덮어쓰지 않는다', async () => {
        const { queryClient } = renderPanel(null)

        await clickAndSettle(graphHeader())
        await deliverSettings(queryClient, { gitSectionsCollapsed: [] })

        expect(graphHeader().getAttribute('aria-expanded')).toBe('false')
    })
})

describe('GitPanel 로빙 포커스', () => {
    const renderWithStash = () => renderPanel({ gitSectionsCollapsed: [] }, { stashes: STASHES, canStash: true })

    test('스크롤 pane 의 마지막 항목에서 ↓ 는 그래프 헤더로 넘어간다', () => {
        renderWithStash()

        fireEvent.keyDown(stashHeader(), { key: 'ArrowDown' })

        expect(document.activeElement).toBe(graphHeader())
    })

    test('그래프 헤더에서 ↑ 는 스크롤 pane 의 마지막 항목으로 돌아온다', () => {
        renderWithStash()

        fireEvent.keyDown(graphHeader(), { key: 'ArrowUp' })

        expect(document.activeElement).toBe(stashHeader())
    })

    test('그래프 헤더에서 ↓ 는 더 갈 곳이 없어 포커스를 옮기지 않는다', () => {
        renderWithStash()
        graphHeader().focus()

        fireEvent.keyDown(graphHeader(), { key: 'ArrowDown' })

        expect(document.activeElement).toBe(graphHeader())
    })

    test('그래프 pane 의 본문에서 누른 ↑↓ 는 가로채지 않는다', () => {
        renderWithStash()

        fireEvent.keyDown(graphPane(), { key: 'ArrowUp' })

        expect(document.activeElement).not.toBe(stashHeader())
    })
})
