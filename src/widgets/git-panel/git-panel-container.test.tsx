import { afterEach, describe, expect, mock, test } from 'bun:test'
import { QueryClient } from '@tanstack/react-query'
import type { AiTextResponse, GitStatus, ProjectId } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { TooltipProvider } from '@shared/ui/tooltip'
import { act, fireEvent, renderWithProviders, screen } from '@shared/testing/render'
import { writeCommitMessageDraft } from '@entities/git/commit-message-memory'

/**
 * The AI commit message half of the wrong-repository guard (audit §1-A): nothing remounts
 * `GitPanelContainer` when the active project changes, so a generation started for one repository
 * used to resolve into whatever repository was on screen by then.
 *
 * The two IPC modules the generation path crosses are stubbed by *spreading* the live module and
 * overriding only what this file drives. `mock.module` registrations are process-global and
 * last-one-wins (`docs/memory/test-conventions.md` §3), and `entities/git/git.query.test.ts`
 * registers its own stubs of these same two modules — spreading keeps every other export that file
 * (and `features/editor`'s AI callers) relies on intact whichever order the two files load in.
 *
 * `@shared/lib/monaco/setup` is stubbed with the shape `git-panel.test.tsx` already uses, since the
 * panel reaches monaco through `commit-detail-panel` → `layout.query` → `tab-path-change`. The
 * container itself is therefore pulled in through a *dynamic* `import()`, after the stubs exist.
 *
 * Git reads are served from a pre-seeded cache rather than from a stubbed transport: every git
 * `queryOptions` factory leaves `staleTime` at the client default, so the client built here sets it
 * to `Infinity` and no query refetches (and fails) on mount. Labels are locale keys because the test
 * i18n instance carries no bundles.
 */
mock.module('@shared/lib/monaco/setup', () => ({ monaco: { Uri: { file: () => ({ toString: () => '' }) }, editor: {} } }))

const actualGitIpc = await import('@entities/git/git.ipc')
const actualAiIpc = await import('@entities/ai/ai.ipc')

const generationControl: { resolve: ((response: AiTextResponse) => void) | null } = { resolve: null }
const cancelledRequestIds: string[] = []

mock.module('@entities/git/git.ipc', () => ({
    ...actualGitIpc,
    getGitDiffStagedText: () => Promise.resolve({ diffText: 'diff --git a/a.ts b/a.ts' }),
}))

mock.module('@entities/ai/ai.ipc', () => ({
    ...actualAiIpc,
    generateAiCommitMessage: () =>
        new Promise<AiTextResponse>((resolve) => {
            generationControl.resolve = resolve
        }),
    cancelAiRequest: (requestId: string) => {
        cancelledRequestIds.push(requestId)
        return Promise.resolve(undefined)
    },
}))

const { GitPanelContainer } = await import('@widgets/git-panel/git-panel-container')

const PROJECT_A: ProjectId = 'project-a'
const PROJECT_B: ProjectId = 'project-b'
const PROJECT_B_DRAFT = 'B 저장소에 쓰던 메시지'
const AI_MESSAGE_FOR_A = 'feat(a): A 저장소의 변경을 요약한 메시지'

/** One change is enough to make the generate button enabled (`canGenerateCommitMessage`). */
const STATUS: GitStatus = {
    rows: [{ path: 'a.ts', absPath: '/tmp/a/a.ts', staged: null, unstaged: 'modified', isConflicted: false }],
    branch: 'main',
    ahead: 0,
    behind: 0,
    hasRemote: false,
}

const MICROTASK_FLUSH_COUNT = 12

const createSeededQueryClient = () => {
    const queryClient = new QueryClient({
        defaultOptions: {
            queries: { retry: 0, gcTime: Infinity, staleTime: Infinity, networkMode: 'always', refetchOnWindowFocus: false },
            mutations: { retry: 0, networkMode: 'always' },
        },
    })
    queryClient.setQueryData(QUERY_KEY.SETTINGS.CURRENT, { gitSectionsCollapsed: [] })
    for (const projectId of [PROJECT_A, PROJECT_B]) {
        queryClient.setQueryData(QUERY_KEY.GIT.STATUS(projectId), STATUS)
        queryClient.setQueryData(QUERY_KEY.GIT.LOG(projectId), [])
        queryClient.setQueryData(QUERY_KEY.GIT.REMOTES(projectId), [])
        queryClient.setQueryData(QUERY_KEY.GIT.BRANCHES(projectId), [])
        queryClient.setQueryData(QUERY_KEY.GIT.STASHES(projectId), [])
        queryClient.setQueryData(QUERY_KEY.GIT.TAGS(projectId), [])
    }
    return queryClient
}

const buildContainer = (projectId: ProjectId) => (
    <TooltipProvider>
        <GitPanelContainer projectId={projectId} />
    </TooltipProvider>
)

/**
 * happy-dom delivers `MutationObserver` records on the microtask queue, so repeated flushes are both
 * enough and more deterministic than waiting on a DOM condition (`test-conventions.md` §5).
 */
const flushMicrotasks = () =>
    act(async () => {
        for (let index = 0; index < MICROTASK_FLUSH_COUNT; index += 1) await Promise.resolve()
    })

const commitMessageInput = () => screen.getByPlaceholderText('git.commitMessagePlaceholder') as HTMLTextAreaElement

const generateButton = () => screen.getByRole('button', { name: 'git.generateCommitMessage' })

afterEach(() => {
    writeCommitMessageDraft(PROJECT_A, '')
    writeCommitMessageDraft(PROJECT_B, '')
    generationControl.resolve = null
    cancelledRequestIds.length = 0
})

describe('GitPanelContainer 의 AI 커밋 메시지 생성', () => {
    test('생성 중 프로젝트가 바뀌면 뒤늦게 도착한 메시지를 새 프로젝트 입력창에 넣지 않는다', async () => {
        writeCommitMessageDraft(PROJECT_B, PROJECT_B_DRAFT)
        const { rerender } = renderWithProviders(buildContainer(PROJECT_A), { queryClient: createSeededQueryClient() })

        fireEvent.click(generateButton())
        await flushMicrotasks()
        expect(generationControl.resolve).not.toBeNull()

        rerender(buildContainer(PROJECT_B))
        expect(commitMessageInput().value).toBe(PROJECT_B_DRAFT)

        generationControl.resolve?.({ requestId: 'request-1', text: AI_MESSAGE_FOR_A })
        await flushMicrotasks()

        expect(commitMessageInput().value).toBe(PROJECT_B_DRAFT)
    })

    test('프로젝트가 바뀌면 남겨진 생성 요청을 취소하고 스피너를 끈다', async () => {
        const { rerender } = renderWithProviders(buildContainer(PROJECT_A), { queryClient: createSeededQueryClient() })

        fireEvent.click(generateButton())
        await flushMicrotasks()
        expect(screen.getByRole('button', { name: 'git.generatingCommitMessage' })).toBeDefined()

        rerender(buildContainer(PROJECT_B))
        expect(generateButton()).toBeDefined()

        await flushMicrotasks()
        expect(cancelledRequestIds).toHaveLength(1)
    })
})
