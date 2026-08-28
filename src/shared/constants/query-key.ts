import type { AiProviderId, AppFileTarget, DiffMode, ProjectId } from '@shared/api/bindings'

const GIT_SCOPE_COMMIT_FILES = 'commit-files'
const GIT_SCOPE_SHOW = 'show'

/**
 * Exported (unlike {@link GIT_SCOPE_COMMIT_FILES}/{@link GIT_SCOPE_SHOW} above, which only this
 * file's own factories need) because `ipc-sync-provider.tsx`'s `fs:changed` handler has to recognize
 * these same two scopes from the *other* side — matching a cached query's key without re-typing the
 * `'gutter'`/`'diff'` string literals there (contract
 * docs/acknowledge/2026-08-27-d44-git-worktree-staleness-contract.md §1).
 */
export const GIT_SCOPE_GUTTER = 'gutter'
export const GIT_SCOPE_DIFF = 'diff'

export const QUERY_KEY = {
    PROJECT: {
        ALL: ['project'] as const,
        LIST: ['project', 'list'] as const,
        RECENT: ['project', 'recent'] as const,
        ACTIVE: ['project', 'active'] as const,
        DETAIL: (projectId: ProjectId) => ['project', 'detail', projectId] as const,
    },
    LAYOUT: {
        ALL: ['layout'] as const,
        DETAIL: (projectId: ProjectId) => ['layout', 'detail', projectId] as const,
    },
    FILE: {
        ALL: ['file'] as const,
        CONTENT: (path: string) => ['file', 'content', path] as const,
        RAW: (path: string) => ['file', 'raw', path] as const,
        MIRRORS: (projectId: ProjectId) => ['file', 'mirrors', projectId] as const,
        UNTITLED_MIRRORS: (projectId: ProjectId) => ['file', 'untitled-mirrors', projectId] as const,
    },
    TREE: {
        ALL: ['tree'] as const,
        ROWS: (projectId: ProjectId) => ['tree', 'rows', projectId] as const,
    },
    SEARCH: {
        ALL: ['search'] as const,
        PROJECT_FILES: (projectId: ProjectId) => ['search', 'project-files', projectId] as const,
    },
    GIT: {
        ALL: ['git'] as const,
        PROJECT: (projectId: ProjectId) => ['git', projectId] as const,
        STATUS: (projectId: ProjectId) => ['git', projectId, 'status'] as const,
        LOG: (projectId: ProjectId) => ['git', projectId, 'log'] as const,
        REMOTES: (projectId: ProjectId) => ['git', projectId, 'remotes'] as const,
        DIFF: (projectId: ProjectId, path: string, mode: DiffMode) => ['git', projectId, GIT_SCOPE_DIFF, path, mode] as const,
        GUTTER: (projectId: ProjectId, path: string) => ['git', projectId, GIT_SCOPE_GUTTER, path] as const,
        CURRENT_USER: (projectId: ProjectId) => ['git', projectId, 'current-user'] as const,
        BRANCHES: (projectId: ProjectId) => ['git', projectId, 'branches'] as const,
        STASHES: (projectId: ProjectId) => ['git', projectId, 'stashes'] as const,
        TAGS: (projectId: ProjectId) => ['git', projectId, 'tags'] as const,
        COMMIT_FILES: (projectId: ProjectId, rev: string) => ['git', projectId, GIT_SCOPE_COMMIT_FILES, rev] as const,
        FILE_LOG: (projectId: ProjectId, path: string) => ['git', projectId, 'file-log', path] as const,
        SHOW: (projectId: ProjectId, rev: string, path: string) => ['git', projectId, GIT_SCOPE_SHOW, rev, path] as const,
        BLAME_LINE: (projectId: ProjectId, path: string, line: number) => ['git', projectId, 'blame-line', path, line] as const,
        BLAME_OVERLAY: (projectId: ProjectId, path: string) => ['git', projectId, 'blame-overlay', path] as const,
        CONFLICT_SIDES: (projectId: ProjectId, path: string) => ['git', projectId, 'conflict-sides', path] as const,
        /**
         * Scopes keyed by an immutable `rev` (commit SHA) rather than by live working-tree/index
         * state — their `staleTime: Infinity` queries (see `git.query.ts`) should survive an
         * unrelated mutation's coarse `PROJECT`-prefix invalidation instead of being refetched for
         * no reason every time the user stages/unstages/stashes/etc. while a commit-detail or
         * file-history panel happens to be open.
         */
        REV_IMMUTABLE_SCOPES: [GIT_SCOPE_COMMIT_FILES, GIT_SCOPE_SHOW] as const,
    },
    LSP: {
        ALL: ['lsp'] as const,
        SERVERS: ['lsp', 'servers'] as const,
        SESSIONS: (projectId: ProjectId) => ['lsp', 'sessions', projectId] as const,
    },
    AGENT: {
        ALL: ['agent'] as const,
        PROJECT: (projectId: ProjectId) => ['agent', 'project', projectId] as const,
        CLI: ['agent', 'cli'] as const,
        /**
         * Prefix covering every `HOOKS(projectId, agentName)` entry for `projectId` regardless of
         * `agentName` — TanStack Query's default partial key matching treats this shorter array as
         * a prefix, so `removeQueries`/`invalidateQueries` against it sweeps all agents at once
         * without enumerating `agentName` values. Exists only for that scope-wide purge (see
         * `PROJECT_SCOPED_KEYS` below); queries themselves are always keyed by the full `HOOKS`.
         */
        HOOKS_PROJECT: (projectId: ProjectId) => ['agent', 'hooks', projectId] as const,
        HOOKS: (projectId: ProjectId, agentName: string) => ['agent', 'hooks', projectId, agentName] as const,
    },
    PLUGIN: {
        ALL: ['plugin'] as const,
        LIST: ['plugin', 'list'] as const,
    },
    TERMINAL: {
        ALL: ['terminal'] as const,
        PROFILES: ['terminal', 'profiles'] as const,
        SESSIONS: (projectId: string) => ['terminal', 'sessions', projectId] as const,
    },
    TASK: {
        ALL: ['task'] as const,
        LIST: (projectId: ProjectId) => ['task', 'list', projectId] as const,
    },
    FONT: {
        ALL: ['font'] as const,
        LIST: ['font', 'list'] as const,
    },
    LOCALE: {
        ALL: ['locale'] as const,
        LIST: ['locale', 'list'] as const,
        CURRENT: ['locale', 'current'] as const,
    },
    THEME: {
        ALL: ['theme'] as const,
        LIST: ['theme', 'list'] as const,
        CURRENT: ['theme', 'current'] as const,
        DETAIL: (themeId: string) => ['theme', 'detail', themeId] as const,
    },
    SETTINGS: {
        ALL: ['settings'] as const,
        CURRENT: ['settings', 'current'] as const,
    },
    APP_FILE: {
        ALL: ['app-file'] as const,
        CONTENT: (target: AppFileTarget) => ['app-file', 'content', target] as const,
    },
    SNIPPET: {
        ALL: ['snippet'] as const,
        LIST: ['snippet', 'list'] as const,
    },
    SYSTEM: {
        ALL: ['system'] as const,
        USAGE: ['system', 'usage'] as const,
        USAGE_BREAKDOWN: ['system', 'usage', 'breakdown'] as const,
    },
    IDE: {
        ALL: ['ide'] as const,
        STATUS: ['ide', 'status'] as const,
    },
    AI: {
        ALL: ['ai'] as const,
        TOKEN_STATUS: ['ai', 'token-status'] as const,
        MODELS: (provider: AiProviderId) => ['ai', 'models', provider] as const,
    },
    SYNC: {
        ALL: ['sync'] as const,
        STATUS: ['sync', 'status'] as const,
    },
    REMOTE: {
        STATUS: ['remote', 'status'] as const,
    },
}

/**
 * Every `QUERY_KEY` factory that scopes its whole subtree to one project — the project's editor
 * layout, git state, tree cache, mirrors, agent roster/hooks, terminal sessions, task list, and LSP
 * sessions all stop being fetchable (or meaningful) the moment that project closes.
 * `IpcSyncProvider`'s `projectClosed` handler (contract F1#4) walks this array with
 * `removeQueries({ queryKey: scopedKey(projectId) })` instead of naming a handful by hand, so a new
 * project-scoped domain only has to add its factory here to be purged. `query-key.test.ts` locks
 * this list's exhaustiveness against a maintained manifest of every `QUERY_KEY` branch.
 *
 * Each entry is deliberately the *coarsest* factory for its domain (e.g. `GIT.PROJECT`, not
 * `GIT.STATUS`) — TanStack Query's default partial key matching treats a shorter key array as a
 * prefix, so removing at the coarse key already removes every finer-grained scope nested under it.
 */
export const PROJECT_SCOPED_KEYS: ReadonlyArray<(projectId: ProjectId) => readonly unknown[]> = [
    QUERY_KEY.PROJECT.DETAIL,
    QUERY_KEY.LAYOUT.DETAIL,
    QUERY_KEY.TREE.ROWS,
    QUERY_KEY.SEARCH.PROJECT_FILES,
    QUERY_KEY.GIT.PROJECT,
    QUERY_KEY.AGENT.PROJECT,
    QUERY_KEY.AGENT.HOOKS_PROJECT,
    QUERY_KEY.TERMINAL.SESSIONS,
    QUERY_KEY.TASK.LIST,
    QUERY_KEY.FILE.MIRRORS,
    QUERY_KEY.FILE.UNTITLED_MIRRORS,
    QUERY_KEY.LSP.SESSIONS,
]

/**
 * `QUERY_KEY.FILE.CONTENT`/`FILE.RAW` are keyed by a bare file `path`, not a `ProjectId`, so they
 * can't be swept by `PROJECT_SCOPED_KEYS`'s `(projectId) => key[]` shape — a closing project's open
 * files otherwise stayed cached under their old path keys forever (contract §1.3(8)).
 * `IpcSyncProvider`'s `projectClosed` handler instead sweeps them with a `predicate` matching any
 * cached query whose key starts with one of these `[domain, scope]` prefixes *and* whose path falls
 * under the closing project's root — see `isProjectRootPathQueryKey` there. `query-key.test.ts`
 * locks this list's exhaustiveness against every path-keyed `QUERY_KEY` leaf the same way
 * `QUERY_KEY_LEAF_CLASSIFICATION` locks `PROJECT_SCOPED_KEYS` above.
 */
export const PROJECT_SCOPED_PATH_KEY_PREFIXES: ReadonlyArray<readonly [string, string]> = [
    ['file', 'content'],
    ['file', 'raw'],
]
