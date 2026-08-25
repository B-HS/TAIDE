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
    PROBE_TIMEOUT_MS: 1500,
    MIN_PORT: 1,
    MAX_PORT: 65535,
} as const

export const REMOTE_GATED_SETTINGS_KEYS = ['shellOverride', 'remotePasswordOnlyLogin', 'remoteAllowedHosts', 'aiOmlxBaseUrl'] as const

export const DEFAULT_TEST_LOCALE = 'en'

export const LSP_SYMBOL_TIMEOUT_MS = 15_000
export const SEARCH_SETTLE_TIMEOUT_MS = 10_000
export const TERMINAL_DECORATION_SOFT_CHECK_TIMEOUT_MS = 5_000

/** Mirrors `src/shared/constants/code-font-size.ts`'s `DEFAULT_CODE_FONT_SIZE` — used only as a fallback when `settings_get` omits `editorFontSize`. */
export const FALLBACK_EDITOR_FONT_SIZE = 13
export const FONT_SIZE_SENTINEL_DELTA = 1

export const KEY_CHORD = {
    SAVE: 'Meta+S',
    SELECT_ALL: 'Meta+A',
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
