import type { AiProviderId, DiffMode, ProjectId } from '@shared/api/bindings'

const GIT_SCOPE_COMMIT_FILES = 'commit-files'
const GIT_SCOPE_SHOW = 'show'

export const QUERY_KEY = {
    APP: {
        ALL: ['app'] as const,
        INFO: ['app', 'info'] as const,
    },
    PROJECT: {
        ALL: ['project'] as const,
        LIST: ['project', 'list'] as const,
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
    GIT: {
        ALL: ['git'] as const,
        PROJECT: (projectId: ProjectId) => ['git', projectId] as const,
        STATUS: (projectId: ProjectId) => ['git', projectId, 'status'] as const,
        LOG: (projectId: ProjectId) => ['git', projectId, 'log'] as const,
        REMOTES: (projectId: ProjectId) => ['git', projectId, 'remotes'] as const,
        DIFF: (projectId: ProjectId, path: string, mode: DiffMode) => ['git', projectId, 'diff', path, mode] as const,
        GUTTER: (projectId: ProjectId, path: string) => ['git', projectId, 'gutter', path] as const,
        CURRENT_USER: (projectId: ProjectId) => ['git', projectId, 'current-user'] as const,
        BRANCHES: (projectId: ProjectId) => ['git', projectId, 'branches'] as const,
        STASHES: (projectId: ProjectId) => ['git', projectId, 'stashes'] as const,
        COMMIT_FILES: (projectId: ProjectId, rev: string) => ['git', projectId, GIT_SCOPE_COMMIT_FILES, rev] as const,
        FILE_LOG: (projectId: ProjectId, path: string) => ['git', projectId, 'file-log', path] as const,
        SHOW: (projectId: ProjectId, rev: string, path: string) => ['git', projectId, GIT_SCOPE_SHOW, rev, path] as const,
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
        INSTALL_PROGRESS: ['lsp', 'install-progress'] as const,
    },
    AGENT: {
        ALL: ['agent'] as const,
        PROJECT: (projectId: ProjectId) => ['agent', 'project', projectId] as const,
        CLI: ['agent', 'cli'] as const,
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
