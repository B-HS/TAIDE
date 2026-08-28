export const LOGIN_PASSWORD_ENV_VAR = 'TAIDE_E2E_PASSWORD'

export const LOGIN_ENDPOINT = '/__taide/login'
export const LOGIN_PROBE_MARKER = 'name="password"'

export const HTTP_STATUS = {
    SEE_OTHER: 303,
    UNAUTHORIZED: 401,
    TOO_MANY_REQUESTS: 429,
    REDIRECT_RANGE_START: 300,
    REDIRECT_RANGE_END_EXCLUSIVE: 400,
} as const

export const PORT_DISCOVERY = {
    LOG_PORT_PATTERN: /원격 접속 서버 기동: port=(\d+)/g,
    PROCESS_NAME_PATTERN: 'taide',
    APP_BUNDLE_EXECUTABLE_PATH_MARKER: '.app/Contents/MacOS/',
    PROBE_TIMEOUT_MS: 1500,
    MIN_PORT: 1,
    MAX_PORT: 65535,
} as const

export const REMOTE_GATED_SETTINGS_KEYS = ['shellOverride', 'remotePasswordOnlyLogin', 'remoteAllowedHosts', 'aiOmlxBaseUrl'] as const

export const DEFAULT_TEST_LOCALE = 'en'

/**
 * Workspace symbol search (`#`) needs the LSP to finish indexing the whole fixture project, not
 * just the one open file document symbols (`@`) need — pilot run observed this taking noticeably
 * longer than the original 15s under this session's load (many fixture projects opened/closed in
 * quick succession churn the LSP process pool), so this is doubled rather than tuned tighter.
 */
export const LSP_SYMBOL_TIMEOUT_MS = 30_000
export const SEARCH_SETTLE_TIMEOUT_MS = 10_000
/**
 * Git status only refreshes after `WATCH_DEBOUNCE_MS`(300ms, `constants.rs`)'s fs-watcher echo
 * reaches the frontend (`fsChanged` → `GIT.PROJECT` invalidation, `ipc-sync-provider.tsx`) — an
 * external `appendFile` (not routed through the app's own save path) has no faster signal. 300ms
 * is generous on an idle system, but the default 5s `toBeVisible()` timeout was observed to be too
 * tight under this session's load (many fixture-project churns queuing watcher/LSP work).
 */
export const GIT_STATUS_SETTLE_TIMEOUT_MS = 15_000
export const TERMINAL_DECORATION_SOFT_CHECK_TIMEOUT_MS = 5_000

/** Mirrors `src/shared/constants/code-font-size.ts`'s `DEFAULT_CODE_FONT_SIZE` — used only as a fallback when `settings_get` omits `editorFontSize`. */
export const FALLBACK_EDITOR_FONT_SIZE = 13
export const FONT_SIZE_SENTINEL_DELTA = 1

export const KEY_CHORD = {
    SAVE: 'Meta+S',
    SELECT_ALL: 'Meta+A',
    PASTE: 'Meta+V',
    QUICK_OPEN: 'Meta+P',
    COMMAND_PALETTE: 'Meta+Shift+P',
    WORKSPACE_SYMBOL: 'Meta+T',
    SEARCH_PANEL: 'Meta+Shift+F',
    GIT_PANEL: 'Control+Shift+G',
    NEW_TERMINAL: 'Control+Shift+Backquote',
} as const

export const PALETTE_MODE_PREFIX = {
    COMMAND: '>',
    SYMBOL: '@',
    LINE: ':',
    WORKSPACE_SYMBOL: '#',
} as const
