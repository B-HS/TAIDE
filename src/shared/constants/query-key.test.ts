import { describe, expect, test } from 'bun:test'
import { PROJECT_SCOPED_KEYS, PROJECT_SCOPED_PATH_KEY_PREFIXES, QUERY_KEY } from '@shared/constants/query-key'

const SENTINEL_PROJECT_ID = 'project-1'

type QueryKeyLeaf =
    { path: string; scopedByProject: false } | { path: string; scopedByProject: true; factory: (projectId: string) => readonly unknown[] }

/**
 * Every leaf under `QUERY_KEY`, hand-classified as either project-scoped (its whole subtree stops
 * being meaningful once the project closes — must be covered by `PROJECT_SCOPED_KEYS`) or not
 * (global, or scoped by something other than a bare `ProjectId`). This manifest is what keeps
 * `PROJECT_SCOPED_KEYS`'s exhaustiveness test meaningful — a reviewer adding a new project-scoped
 * `QUERY_KEY` branch must add it here too, or the "every covered leaf is listed" assertion below
 * will not know to demand it.
 */
const QUERY_KEY_LEAF_CLASSIFICATION: QueryKeyLeaf[] = [
    { path: 'APP.ALL', scopedByProject: false },
    { path: 'APP.INFO', scopedByProject: false },
    { path: 'PROJECT.ALL', scopedByProject: false },
    { path: 'PROJECT.LIST', scopedByProject: false },
    { path: 'PROJECT.ACTIVE', scopedByProject: false },
    { path: 'PROJECT.DETAIL', scopedByProject: true, factory: QUERY_KEY.PROJECT.DETAIL },
    { path: 'LAYOUT.ALL', scopedByProject: false },
    { path: 'LAYOUT.DETAIL', scopedByProject: true, factory: QUERY_KEY.LAYOUT.DETAIL },
    { path: 'FILE.ALL', scopedByProject: false },
    /**
     * Keyed by a bare file path, not a `ProjectId` — `scopedByProject: false` here only means "not
     * covered by `PROJECT_SCOPED_KEYS`'s `(projectId) => key[]` shape". Both are still project-scoped
     * in the sense that matters (must be swept when their project closes) via the separate
     * `PROJECT_SCOPED_PATH_KEY_PREFIXES` mechanism — see the `PROJECT_SCOPED_PATH_KEY_PREFIXES`
     * `describe` block below.
     */
    { path: 'FILE.CONTENT', scopedByProject: false },
    { path: 'FILE.RAW', scopedByProject: false },
    { path: 'FILE.MIRRORS', scopedByProject: true, factory: QUERY_KEY.FILE.MIRRORS },
    { path: 'FILE.UNTITLED_MIRRORS', scopedByProject: true, factory: QUERY_KEY.FILE.UNTITLED_MIRRORS },
    { path: 'TREE.ALL', scopedByProject: false },
    { path: 'TREE.ROWS', scopedByProject: true, factory: QUERY_KEY.TREE.ROWS },
    { path: 'GIT.ALL', scopedByProject: false },
    { path: 'GIT.PROJECT', scopedByProject: true, factory: QUERY_KEY.GIT.PROJECT },
    { path: 'GIT.STATUS', scopedByProject: false },
    { path: 'GIT.LOG', scopedByProject: false },
    { path: 'GIT.REMOTES', scopedByProject: false },
    { path: 'GIT.DIFF', scopedByProject: false },
    { path: 'GIT.GUTTER', scopedByProject: false },
    { path: 'GIT.CURRENT_USER', scopedByProject: false },
    { path: 'GIT.BRANCHES', scopedByProject: false },
    { path: 'GIT.STASHES', scopedByProject: false },
    { path: 'GIT.TAGS', scopedByProject: false },
    { path: 'GIT.COMMIT_FILES', scopedByProject: false },
    { path: 'GIT.FILE_LOG', scopedByProject: false },
    { path: 'GIT.SHOW', scopedByProject: false },
    { path: 'GIT.REV_IMMUTABLE_SCOPES', scopedByProject: false },
    { path: 'GIT.BLAME_LINE', scopedByProject: false },
    { path: 'GIT.BLAME_OVERLAY', scopedByProject: false },
    { path: 'GIT.CONFLICT_SIDES', scopedByProject: false },
    { path: 'LSP.ALL', scopedByProject: false },
    { path: 'LSP.SERVERS', scopedByProject: false },
    { path: 'LSP.SESSIONS', scopedByProject: true, factory: QUERY_KEY.LSP.SESSIONS },
    { path: 'AGENT.ALL', scopedByProject: false },
    { path: 'AGENT.PROJECT', scopedByProject: true, factory: QUERY_KEY.AGENT.PROJECT },
    { path: 'AGENT.CLI', scopedByProject: false },
    { path: 'AGENT.HOOKS_PROJECT', scopedByProject: true, factory: QUERY_KEY.AGENT.HOOKS_PROJECT },
    { path: 'AGENT.HOOKS', scopedByProject: false },
    { path: 'PLUGIN.ALL', scopedByProject: false },
    { path: 'PLUGIN.LIST', scopedByProject: false },
    { path: 'TERMINAL.ALL', scopedByProject: false },
    { path: 'TERMINAL.PROFILES', scopedByProject: false },
    { path: 'TERMINAL.SESSIONS', scopedByProject: true, factory: QUERY_KEY.TERMINAL.SESSIONS },
    { path: 'TASK.ALL', scopedByProject: false },
    { path: 'TASK.LIST', scopedByProject: true, factory: QUERY_KEY.TASK.LIST },
    { path: 'FONT.ALL', scopedByProject: false },
    { path: 'FONT.LIST', scopedByProject: false },
    { path: 'LOCALE.ALL', scopedByProject: false },
    { path: 'LOCALE.LIST', scopedByProject: false },
    { path: 'LOCALE.CURRENT', scopedByProject: false },
    { path: 'THEME.ALL', scopedByProject: false },
    { path: 'THEME.LIST', scopedByProject: false },
    { path: 'THEME.CURRENT', scopedByProject: false },
    { path: 'THEME.DETAIL', scopedByProject: false },
    { path: 'SETTINGS.ALL', scopedByProject: false },
    { path: 'SETTINGS.CURRENT', scopedByProject: false },
    { path: 'APP_FILE.ALL', scopedByProject: false },
    { path: 'APP_FILE.CONTENT', scopedByProject: false },
    { path: 'SNIPPET.ALL', scopedByProject: false },
    { path: 'SNIPPET.LIST', scopedByProject: false },
    { path: 'SYSTEM.ALL', scopedByProject: false },
    { path: 'SYSTEM.USAGE', scopedByProject: false },
    { path: 'SYSTEM.USAGE_BREAKDOWN', scopedByProject: false },
    { path: 'IDE.ALL', scopedByProject: false },
    { path: 'IDE.STATUS', scopedByProject: false },
    { path: 'AI.ALL', scopedByProject: false },
    { path: 'AI.TOKEN_STATUS', scopedByProject: false },
    { path: 'AI.MODELS', scopedByProject: false },
    { path: 'SYNC.ALL', scopedByProject: false },
    { path: 'SYNC.STATUS', scopedByProject: false },
    { path: 'REMOTE.STATUS', scopedByProject: false },
]

describe('QUERY_KEY_LEAF_CLASSIFICATION', () => {
    test('QUERY_KEY 의 모든 도메인이 분류표에 등장한다 — 새 도메인 추가 시 분류 누락을 방지', () => {
        const classifiedDomains = new Set(QUERY_KEY_LEAF_CLASSIFICATION.map((leaf) => leaf.path.split('.')[0]))
        expect(classifiedDomains).toEqual(new Set(Object.keys(QUERY_KEY)))
    })

    test('QUERY_KEY 의 모든 리프가 분류표에 등장한다 — 새 리프 추가 시 분류 누락을 방지', () => {
        const classifiedLeaves = new Set(QUERY_KEY_LEAF_CLASSIFICATION.map((leaf) => leaf.path))
        const actualLeaves = new Set(Object.entries(QUERY_KEY).flatMap(([domain, branch]) => Object.keys(branch).map((leaf) => `${domain}.${leaf}`)))
        expect(classifiedLeaves).toEqual(actualLeaves)
    })
})

describe('PROJECT_SCOPED_KEYS', () => {
    const projectScopedLeaves = QUERY_KEY_LEAF_CLASSIFICATION.filter(
        (leaf): leaf is Extract<QueryKeyLeaf, { scopedByProject: true }> => leaf.scopedByProject,
    )

    test('분류표가 project-scoped 로 표시한 리프 전수를 포함한다 — 과소 커버리지 방지', () => {
        expect(PROJECT_SCOPED_KEYS).toHaveLength(projectScopedLeaves.length)
        for (const leaf of projectScopedLeaves) expect(PROJECT_SCOPED_KEYS).toContain(leaf.factory)
    })

    test('모든 항목이 projectId 를 접두사로 갖는 배열을 만든다', () => {
        for (const scopedKey of PROJECT_SCOPED_KEYS) {
            const key = scopedKey(SENTINEL_PROJECT_ID)
            expect(Array.isArray(key)).toBe(true)
            expect(key).toContain(SENTINEL_PROJECT_ID)
        }
    })

    test('GIT.PROJECT 는 세부 git 스코프 전체를 접두사로 포함한다 — 개별 등록 불필요를 확인', () => {
        const gitProjectKey = QUERY_KEY.GIT.PROJECT(SENTINEL_PROJECT_ID)
        const gitStatusKey = QUERY_KEY.GIT.STATUS(SENTINEL_PROJECT_ID)
        expect(gitStatusKey.slice(0, gitProjectKey.length)).toEqual([...gitProjectKey])
    })

    test('GIT.BLAME_LINE·BLAME_OVERLAY·CONFLICT_SIDES 도 GIT.PROJECT 를 접두사로 포함한다 — editor-pane 신설 스코프의 프로젝트 정리 커버리지', () => {
        const gitProjectKey = QUERY_KEY.GIT.PROJECT(SENTINEL_PROJECT_ID)
        const scopedKeys = [
            QUERY_KEY.GIT.BLAME_LINE(SENTINEL_PROJECT_ID, 'a.ts', 1),
            QUERY_KEY.GIT.BLAME_OVERLAY(SENTINEL_PROJECT_ID, 'a.ts'),
            QUERY_KEY.GIT.CONFLICT_SIDES(SENTINEL_PROJECT_ID, 'a.ts'),
        ]
        for (const scopedKey of scopedKeys) expect(scopedKey.slice(0, gitProjectKey.length)).toEqual([...gitProjectKey])
    })

    test('AGENT.HOOKS_PROJECT 는 agentName 과 무관하게 AGENT.HOOKS 전체를 접두사로 포함한다', () => {
        const hooksProjectKey = QUERY_KEY.AGENT.HOOKS_PROJECT(SENTINEL_PROJECT_ID)
        const hooksKey = QUERY_KEY.AGENT.HOOKS(SENTINEL_PROJECT_ID, 'claude')
        expect(hooksKey.slice(0, hooksProjectKey.length)).toEqual([...hooksProjectKey])
    })
})

/**
 * Every `QUERY_KEY` leaf whose factory takes a bare `path`/`target` (never a `ProjectId`) as its
 * sole argument, hand-classified as project-scoped-by-path (must appear in
 * `PROJECT_SCOPED_PATH_KEY_PREFIXES`) or not. `APP_FILE.CONTENT` keys settings/app-level content by
 * an `AppFileTarget` (e.g. `{ kind: 'settings' }`), never a project file path, so it's excluded —
 * closing a project must not sweep the settings screen's own cache entry.
 */
const PATH_KEYED_LEAF_CLASSIFICATION: { path: string; prefix: readonly [string, string] | null }[] = [
    { path: 'FILE.CONTENT', prefix: ['file', 'content'] },
    { path: 'FILE.RAW', prefix: ['file', 'raw'] },
    { path: 'APP_FILE.CONTENT', prefix: null },
]

describe('PROJECT_SCOPED_PATH_KEY_PREFIXES', () => {
    const scopedPrefixes = PATH_KEYED_LEAF_CLASSIFICATION.map((leaf) => leaf.prefix).filter((prefix) => prefix !== null)

    test('분류표가 project-scoped-by-path 로 표시한 리프 전수를 포함한다 — 과소 커버리지 방지', () => {
        expect(PROJECT_SCOPED_PATH_KEY_PREFIXES).toHaveLength(scopedPrefixes.length)
        for (const prefix of scopedPrefixes) expect(PROJECT_SCOPED_PATH_KEY_PREFIXES).toContainEqual(prefix)
    })

    test('각 prefix 는 해당 QUERY_KEY 팩토리가 실제로 만드는 키의 앞 두 요소와 일치한다', () => {
        expect(QUERY_KEY.FILE.CONTENT('a.ts').slice(0, 2)).toEqual(['file', 'content'])
        expect(QUERY_KEY.FILE.RAW('a.ts').slice(0, 2)).toEqual(['file', 'raw'])
    })
})
