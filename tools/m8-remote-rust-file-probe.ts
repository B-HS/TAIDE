import { chromium, expect } from '@playwright/test'
import { z } from 'zod'
import { resolve } from 'node:path'

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const DEADLINE_MS = 10_000
const QUIET_MS = 1_100
const RESTART_CODE = 1012
const HTTP_BAD_REQUEST = 400
const HTTP_NOT_FOUND = 404
const ARG_INDEX = 2
const MODE_INDEX = 3
const SHARED_VIEWS = 2
const SAVED_LOADS = 2
const RECOVERED_LOADS = 3
const FINAL_LOADS = 4
const FINAL_UPGRADES = 2
const FINAL_READS = 5
const FINAL_SAVES = 3
const PREPARED_READS = 4
const PREPARED_SAVES = 2
const FILE_INDENT_SIZE = 2
const BASE_TAB_SIZE = 4
const CHOICE_READS = 5
const DIRTY_FLUSHES = 2
const OWNER_READS = 6
const OWNER_DIRTY_REQUESTS = 5
const TYPED_TEXT = 'typed '
const PATH = 'C:\\synthetic\\file.rs'
const PROJECT = 'prj-synthetic'
const LAYOUT_SCHEMA_VERSION = 2
const MIRROR_READS = 3
const RESTORED_TEXT = 'recovered draft'
const LATEST_MIRROR_TEXT = 'latest draft'
const FINAL_MIRROR_WRITES = 4
const UNMOUNT_MIRROR_WRITES = 3
const PREFERENCE_STAGE = { LANGUAGE: 2, FAILED: 3, THEME: 4, CLOSED: 5, RETRIED: 6, DRAINED: 7 } as const
const PREFERENCE_UPDATES = 4
const PREFERENCE_THEMES = 3
const CATALOG_HELD_READS = 2
const CATALOG_REMOUNT_READS = 3
const CATALOG_FINAL_READS = 4
const THEME_SAVES = 3
const THEME_DELETES = 1
const APP_FILE_WRITES = 2
const CODE_FONT_SIZE = 18
const CODE_WRITES = 5
const KEYBINDING_WRITES = 4
const SNIPPET_SAVES = 5
const SNIPPET_BODY = 'fn synthetic() {}'
const CODE_VIEWPORT = { width: 1100, height: 1000 }
const POPUP_POLL_MS = 16
const FONT_FIXTURES = {
    'mono.ttf':
        '/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint_default_fonts-0.36.2/fonts/Hack-Regular.ttf',
    'sans.ttf':
        '/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/epaint_default_fonts-0.36.2/fonts/Ubuntu-Light.ttf',
} as const
const fixture = process.argv[ARG_INDEX]
if (!fixture || !/^\/private\/tmp\/taide-m8-wasm-tools\.[A-Za-z0-9]+\/file-built$/.test(fixture)) {
    throw new Error('Use the dedicated generated file consumer probe directory')
}
const mode = z
    .enum([
        'continuous',
        'prepared-config',
        'disk-choice',
        'dirty-owner',
        'dirty-close',
        'mirror-restore',
        'mirror-write',
        'application-pump',
        'application-close',
        'canvas',
        'canvas-visual',
        'canvas-abort',
        'preferences',
        'settings-catalog',
        'settings-visual',
        'theme-operations',
        'settings-tooltip',
        'settings-toast',
        'settings-preferences',
        'settings-folders',
        'settings-preview',
        'settings-app-file',
        'settings-code',
        'settings-resource-cache',
        'settings-font-visual',
        'settings-code-style',
        'settings-picker-lifecycle',
        'settings-popup-presence',
        'settings-keybindings',
        'settings-snippets',
    ])
    .parse(process.argv[MODE_INDEX] ?? 'continuous')
const isPreparedConfig = mode === 'prepared-config'
const isDiskChoice = mode === 'disk-choice'
const isDirtyOwner = mode === 'dirty-owner'
const isDirtyClose = mode === 'dirty-close'
const isMirrorRestore = mode === 'mirror-restore'
const isMirrorWrite = mode === 'mirror-write'
const isApplicationPump = mode === 'application-pump'
const isApplicationClose = mode === 'application-close'
const isCanvasVisual = mode === 'canvas-visual'
const isCanvasAbort = mode === 'canvas-abort'
const isCanvas = mode === 'canvas' || isCanvasVisual || isCanvasAbort
const isPreferences = mode === 'preferences'
const isSettingsVisual = mode === 'settings-visual'
const isThemeOperations = mode === 'theme-operations'
const isSettingsTooltip = mode === 'settings-tooltip'
const isSettingsToast = mode === 'settings-toast'
const isSettingsPreferences = mode === 'settings-preferences'
const isSettingsFolders = mode === 'settings-folders'
const isSettingsPreview = mode === 'settings-preview'
const isSettingsAppFile = mode === 'settings-app-file'
const isSettingsCode = mode === 'settings-code'
const isSettingsResourceCache = mode === 'settings-resource-cache'
const isSettingsFontVisual = mode === 'settings-font-visual'
const isSettingsCodeStyle = mode === 'settings-code-style'
const isSettingsPickerLifecycle = mode === 'settings-picker-lifecycle'
const isSettingsPopupPresence = mode === 'settings-popup-presence'
const isSettingsKeybindings = mode === 'settings-keybindings'
const isSettingsSnippets = mode === 'settings-snippets'
const hasThemeCommands =
    isThemeOperations ||
    isSettingsTooltip ||
    isSettingsToast ||
    isSettingsPreferences ||
    isSettingsFolders ||
    isSettingsPreview ||
    isSettingsAppFile ||
    isSettingsCode ||
    isSettingsResourceCache ||
    isSettingsFontVisual ||
    isSettingsCodeStyle ||
    isSettingsPickerLifecycle ||
    isSettingsPopupPresence ||
    isSettingsKeybindings ||
    isSettingsSnippets
const isSettingsCatalog = mode === 'settings-catalog' || isSettingsVisual || hasThemeCommands || isSettingsAppFile
const isApplication = isApplicationPump || isApplicationClose || isCanvas || isPreferences || isSettingsCatalog
const requestSchema = z.object({ seq: z.number().int().nonnegative(), command: z.string(), args: z.record(z.string(), z.unknown()).nullable() })
const stateSchema = z.object({
    connected: z.boolean(),
    ready: z.boolean(),
    documents: z.number().int().nonnegative(),
    views: z.number().int().nonnegative(),
    text: z.string().nullable(),
    dirty: z.boolean().nullable(),
    readOnly: z.boolean().nullable(),
    conflict: z.boolean().nullable(),
    rendered: z.array(z.string()),
    loaded: z.number().int().nonnegative(),
    saves: z.array(z.boolean()),
    saveFailures: z.array(z.string()),
    choices: z.array(z.boolean()),
    choiceFailures: z.array(z.string()),
    dirtyFlushes: z.number().int().nonnegative(),
    dirtyFailures: z.array(z.string()),
    dirtyFlushed: z.boolean(),
    scopeReady: z.boolean(),
    mirrorReady: z.boolean(),
    mirrorTexts: z.array(z.string()).nullable(),
    restored: z.number().int().nonnegative(),
    mirrorRestoreFailures: z.array(z.string()),
    mirrorFailures: z.array(z.string()),
    mirrorStatus: z.string(),
    mirrorTimerArmed: z.boolean(),
    recoveries: z.number().int().nonnegative(),
    responses: z.array(z.number().int().nonnegative()),
    wakes: z.number().int().nonnegative(),
})
const colorKeys = [
    'app.background',
    'app.foreground',
    'appSidebar.background',
    'appSidebar.iconDefault',
    'app.border',
    'app.focusBorder',
    'tabBar.tabActiveBackground',
    'tabBar.tabInactiveBackground',
    'tabBar.tabActiveIndicator',
    'editor.background',
    'editor.foreground',
    'editor.lineNumber',
    'editor.selection',
    'editor.cursor',
    'editor.lineHighlight',
    'statusIndicator.error',
    'statusIndicator.warning',
]
const settingsSchema = z.object({ language: z.string(), editorFontSize: z.number() }).passthrough()
let settings: z.infer<typeof settingsSchema> | null = null
const catalogThemeSchema = z.object({
    id: z.string(),
    name: z.string(),
    type: z.enum(['dark', 'light']),
    colors: z.record(z.string(), z.string()),
    syntax: z.record(z.string(), z.unknown()),
    terminal: z.record(z.string(), z.string()),
})
let catalogTheme: z.infer<typeof catalogThemeSchema> | null = null
let themeLists = 0
let localeLists = 0
let isCatalogHeld = false
let deferredCatalog = Array<() => void>()
let isResourceHeld = isSettingsResourceCache
let deferredResources = Array<() => void>()
let resourceCounts = { font_list: 0, shell_profiles: 0 }
let broadcastTheme: (() => void) | null = null
const storedThemeSchema = z
    .object({
        version: z.literal(1),
        id: z.literal('synthetic'),
        name: z.string(),
        type: z.literal('dark'),
        extends: z.string(),
        colors: z.record(z.string(), z.string()),
        syntax: z.record(z.string(), z.object({ fg: z.string(), bold: z.boolean(), italic: z.boolean() })),
        terminal: z.record(z.string(), z.string()),
    })
    .passthrough()
let storedTheme: z.infer<typeof storedThemeSchema> | null = null
let themeSaves = 0
let themeDeletes = 0
let isThemeHeld = false
let shouldRejectTheme = false
let deferredThemes = Array<() => void>()
const snippetEntrySchema = z.object({
    prefix: z.union([z.string(), z.array(z.string())]),
    body: z.union([z.string(), z.array(z.string())]),
    description: z.union([z.string(), z.array(z.string())]).optional(),
    scope: z.string().optional(),
})
const snippetFileSchema = z.object({ fileName: z.string(), snippets: z.record(z.string(), snippetEntrySchema) })
let snippetFiles = Array<z.infer<typeof snippetFileSchema>>()
let isSnippetHeld = false
let shouldRejectSnippet = false
let deferredSnippets = Array<() => void>()
let disk = isPreparedConfig ? 'disk  \r\n' : 'disk'
let shouldCleanFile = false
let modified = 1
let upgrades = 0
let active = 0
let isReadHeld = false
let isSaveHeld = false
let isWriteHeld = false
let deferredWrite = Array<() => void>()
let mirrorWrites = 0
let mirrorClears = 0
const mirrorEntrySchema = z.object({
    path: z.literal(PATH),
    content: z.string(),
    savedAtMs: z.number(),
    diskModifiedMs: z.number().nullable(),
    conflict: z.boolean(),
    sourceMissing: z.boolean(),
})
const receiptSchema = z.object({ writeId: z.string(), entry: mirrorEntrySchema })
let currentMirror: z.infer<typeof receiptSchema> | null = null
let isPreferenceHeld = false
let shouldRejectPreference = false
let deferredPreferences = Array<() => void>()
let broadcastSettings: (() => void) | undefined
let deferredRead = Array<() => void>()
let deferredSave = Array<() => void>()
let isDirtyHeld = false
let shouldFailDirty = false
let deferredDirty = Array<() => void>()
let isMirrorHeld = isMirrorRestore
let deferredMirror = Array<() => void>()
let mirrorReads = 0
let requests = Array<z.infer<typeof requestSchema>>()
let isAppFileOpen = false
let isAppFileDirty = false
let appFileRevision = 0
let appFileContent = ''
let shouldRejectAppFile = true
const settingsLayout = () => ({
    version: LAYOUT_SCHEMA_VERSION,
    revision: appFileRevision,
    root: {
        node: 'leaf',
        id: 'synthetic-pane',
        tabs: [
            { id: 'synthetic-settings', kind: { kind: 'settings' }, title: 'Settings', pinned: false, preview: false, dirty: false },
            ...(isAppFileOpen
                ? [
                      {
                          id: 'synthetic-app-file',
                          kind: { kind: 'appFile', target: { kind: 'settings' } },
                          title: 'settings.json',
                          pinned: false,
                          preview: false,
                          dirty: isAppFileDirty,
                      },
                  ]
                : []),
        ],
        active: isAppFileOpen ? 'synthetic-app-file' : 'synthetic-settings',
    },
    focusedPane: 'synthetic-pane',
})
const server = Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch: async (request, server) => {
        const url = new URL(request.url)
        if (url.pathname === '/__taide/ws') {
            upgrades += 1
            if (server.upgrade(request)) return
            return new Response(null, { status: HTTP_BAD_REQUEST })
        }
        if (url.pathname === '/__fixture/settings' && request.method === 'POST') {
            const value = settingsSchema.parse(await request.json())
            settings = isPreparedConfig
                ? {
                      ...value,
                      editorTabSize: BASE_TAB_SIZE,
                      editorInsertSpaces: true,
                      editorDetectIndentation: false,
                      trimTrailingWhitespaceOnSave: true,
                      insertFinalNewlineOnSave: true,
                  }
                : value
            return new Response(null)
        }
        if (url.pathname === '/__fixture/theme' && request.method === 'POST' && isSettingsCatalog) {
            catalogTheme = catalogThemeSchema.parse(await request.json())
            return new Response(null)
        }
        if (url.pathname === '/__fixture/control' && request.method === 'POST') {
            const command = z
                .enum([
                    'hold-read',
                    'release-read',
                    'hold-save',
                    'release-save',
                    'enable-cleanup',
                    'external-disk',
                    'hold-dirty',
                    'release-dirty',
                    'fail-dirty',
                    'hold-mirror',
                    'release-mirror',
                    'hold-write',
                    'release-write',
                    'hold-preference',
                    'release-preference',
                    'reject-preference',
                    'external-preference',
                    'hold-catalog',
                    'release-catalog',
                    'theme-event',
                    'hold-theme',
                    'release-theme',
                    'reject-theme',
                    'release-resources',
                    'hold-snippet',
                    'release-snippet',
                    'reject-snippet',
                ])
                .parse(await request.json())
            if (command === 'hold-snippet') isSnippetHeld = true
            if (command === 'release-snippet' || command === 'reject-snippet') {
                isSnippetHeld = false
                shouldRejectSnippet = command === 'reject-snippet'
                for (const release of deferredSnippets) release()
                deferredSnippets = []
            }
            if (command === 'release-resources') {
                isResourceHeld = false
                for (const release of deferredResources) release()
                deferredResources = []
            }
            if (command === 'hold-theme') isThemeHeld = true
            if (command === 'release-theme' || command === 'reject-theme') {
                isThemeHeld = false
                shouldRejectTheme = command === 'reject-theme'
                for (const release of deferredThemes) release()
                deferredThemes = []
            }
            if (command === 'hold-catalog') isCatalogHeld = true
            if (command === 'release-catalog') {
                isCatalogHeld = false
                for (const release of deferredCatalog) release()
                deferredCatalog = []
            }
            if (command === 'theme-event') {
                if (!broadcastTheme) throw new Error('Synthetic theme connection is not ready')
                broadcastTheme()
            }
            if (command === 'hold-preference') isPreferenceHeld = true
            if (command === 'release-preference' || command === 'reject-preference') {
                isPreferenceHeld = false
                shouldRejectPreference = command === 'reject-preference'
                for (const release of deferredPreferences) release()
                deferredPreferences = []
            }
            if (command === 'external-preference') {
                if (!settings) throw new Error('Missing synthetic settings')
                settings = { ...settings, language: 'en' }
                if (!broadcastSettings) throw new Error('Synthetic connection is not ready')
                broadcastSettings()
            }
            if (command === 'hold-write') isWriteHeld = true
            if (command === 'release-write') {
                isWriteHeld = false
                for (const release of deferredWrite) release()
                deferredWrite = []
            }
            if (command === 'hold-mirror') isMirrorHeld = true
            if (command === 'release-mirror') {
                isMirrorHeld = false
                for (const release of deferredMirror) release()
                deferredMirror = []
            }
            if (command === 'hold-dirty') isDirtyHeld = true
            if (command === 'fail-dirty') shouldFailDirty = true
            if (command === 'release-dirty') {
                isDirtyHeld = false
                for (const release of deferredDirty) release()
                deferredDirty = []
            }
            if (command === 'external-disk') {
                disk = `external ${modified}`
                modified += 1
            }
            if (command === 'enable-cleanup') shouldCleanFile = true
            if (command === 'hold-read') isReadHeld = true
            if (command === 'hold-save') isSaveHeld = true
            if (command === 'release-read') {
                isReadHeld = false
                for (const release of deferredRead) release()
                deferredRead = []
            }
            if (command === 'release-save') {
                isSaveHeld = false
                for (const release of deferredSave) release()
                deferredSave = []
            }
            return new Response(null)
        }
        if (url.pathname === '/') {
            if (isSettingsCatalog) return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe/settings.html')))
            if (isCanvas) return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe/canvas.html')))
            const html = isApplication ? 'application.html' : 'files.html'
            return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe', html)))
        }
        const name = url.pathname.slice(1)
        if (isSettingsFontVisual && (name === 'mono.ttf' || name === 'sans.ttf')) return new Response(Bun.file(FONT_FIXTURES[name]))
        if (name === 'probe.js' || name === 'probe_bg.wasm') return new Response(Bun.file(resolve(fixture, name)))
        return new Response(null, { status: HTTP_NOT_FOUND })
    },
    websocket: {
        open: () => {
            active += 1
        },
        close: () => {
            active -= 1
        },
        message: (socket, message) => {
            broadcastTheme = () => socket.send(JSON.stringify({ t: 'event', event: 'theme:changed', payload: '{}' }))
            broadcastSettings = () => socket.send(JSON.stringify({ t: 'event', event: 'settings:changed', payload: JSON.stringify({ settings }) }))
            if (typeof message !== 'string') throw new Error('Only synthetic JSON requests are accepted')
            const request = requestSchema.parse(JSON.parse(message))
            requests = [...requests, request]
            if (!settings) throw new Error('Synthetic settings are missing')
            const reply = (payload: z.infer<typeof requestSchema>['args']) =>
                socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload }))
            const preferenceReply = (response: string) => {
                if (isPreferenceHeld) {
                    deferredPreferences = [...deferredPreferences, () => socket.send(response)]
                    return
                }
                socket.send(response)
            }
            if (request.command === 'snippet_list') {
                expect(request.args).toBeNull()
                socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: snippetFiles }))
                return
            }
            if (request.command === 'snippet_save') {
                expect(isSettingsSnippets).toBe(true)
                const args = z.object({ fileName: z.string(), content: z.string() }).strict().parse(request.args)
                const file = snippetFileSchema.parse({ fileName: args.fileName, snippets: JSON.parse(args.content) })
                const commit = () => {
                    if (shouldRejectSnippet) {
                        shouldRejectSnippet = false
                        socket.send(
                            JSON.stringify({
                                t: 'resp',
                                seq: request.seq,
                                ok: false,
                                payload: { code: 'InvalidArgument', message: 'Synthetic snippet refusal' },
                            }),
                        )
                        return
                    }
                    snippetFiles = [...snippetFiles.filter((stored) => stored.fileName !== file.fileName), file]
                    reply(file)
                }
                if (isSnippetHeld) deferredSnippets = [...deferredSnippets, commit]
                else commit()
                return
            }
            if (request.command === 'snippet_delete') {
                expect(isSettingsSnippets).toBe(true)
                const args = z.object({ fileName: z.string() }).strict().parse(request.args)
                snippetFiles = snippetFiles.filter((file) => file.fileName !== args.fileName)
                reply(null)
                return
            }
            if (isSettingsAppFile && request.command === 'layout_open_tab') {
                expect(request.args).toEqual({
                    projectId: PROJECT,
                    kind: { kind: 'appFile', target: { kind: 'settings' } },
                    title: 'settings.json',
                    target: 'synthetic-pane',
                    preview: false,
                })
                isAppFileOpen = true
                appFileRevision += 1
                appFileContent = JSON.stringify(settings)
                reply(settingsLayout())
                return
            }
            if (isSettingsAppFile && request.command === 'app_file_read') {
                expect(request.args).toEqual({ target: { kind: 'settings' } })
                socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: appFileContent }))
                return
            }
            if (isSettingsAppFile && request.command === 'app_file_write') {
                const args = z
                    .object({ target: z.object({ kind: z.literal('settings') }).strict(), content: z.string() })
                    .strict()
                    .parse(request.args)
                const patch = z.object({ editorTabSize: z.number().int().positive() }).strict().parse(JSON.parse(args.content))
                const nextSettings = { ...settings, ...patch }
                const commit = () => {
                    if (shouldRejectAppFile) {
                        shouldRejectAppFile = false
                        socket.send(
                            JSON.stringify({
                                t: 'resp',
                                seq: request.seq,
                                ok: false,
                                payload: { code: 'InvalidArgument', message: 'Synthetic settings refusal' },
                            }),
                        )
                        return
                    }
                    settings = nextSettings
                    appFileContent = `${JSON.stringify(nextSettings)}\n`
                    if (!broadcastSettings) throw new Error('Missing synthetic settings listener')
                    broadcastSettings()
                    reply(null)
                }
                if (isWriteHeld) {
                    deferredWrite = [...deferredWrite, commit]
                    return
                }
                commit()
                return
            }
            if (request.command === 'system_open_app_data_path') {
                expect(isSettingsFolders).toBe(true)
                z.object({ kind: z.literal('themes') })
                    .strict()
                    .parse(request.args)
                socket.send(
                    JSON.stringify({
                        t: 'resp',
                        seq: request.seq,
                        ok: false,
                        payload: {
                            code: 'Localized',
                            message: {
                                kind: 'Forbidden',
                                key: 'error.remote.deniedUnreachableDesktopWindow',
                                args: { command: request.command },
                                fallback: `a remote session cannot open or control a window or app shown on the desktop's own display: ${request.command}`,
                            },
                        },
                    }),
                )
                return
            }
            if (request.command === 'font_list' || request.command === 'shell_profiles') {
                expect(isSettingsCatalog).toBe(true)
                expect(request.args).toBeNull()
                const resources = {
                    font_list: [
                        { name: 'Synthetic Mono', monospaced: true },
                        { name: 'Synthetic Sans', monospaced: false },
                    ],
                    shell_profiles: [
                        { id: 'synthetic-shell', name: 'Synthetic shell', path: '/synthetic/shell', args: [] },
                        { id: 'synthetic-other', name: 'Other shell', path: '/synthetic/other', args: [] },
                    ],
                }
                resourceCounts = { ...resourceCounts, [request.command]: resourceCounts[request.command] + 1 }
                const resource = resources[request.command]
                const fail = isSettingsResourceCache && request.command === 'shell_profiles' && resourceCounts.shell_profiles === 1
                const finish = () =>
                    socket.send(
                        JSON.stringify({
                            t: 'resp',
                            seq: request.seq,
                            ok: !fail,
                            payload: fail ? { code: 'Internal', message: 'Synthetic shell refusal' } : resource,
                        }),
                    )
                if (isResourceHeld) deferredResources = [...deferredResources, finish]
                else finish()
                return
            }
            if (request.command === 'theme_list' || request.command === 'locale_list') {
                expect(isSettingsCatalog).toBe(true)
                expect(request.args).toBeNull()
                const catalogReplies = {
                    theme_list: () => {
                        themeLists += 1
                        const id = !hasThemeCommands && themeLists === CATALOG_HELD_READS ? 'stale-theme' : 'taide-dark'
                        const builtin = { id, name: 'Synthetic theme', type: 'dark', builtin: true }
                        if (hasThemeCommands && storedTheme)
                            return {
                                ok: true,
                                payload: [builtin, { id: storedTheme.id, name: storedTheme.name, type: storedTheme.type, builtin: false }],
                            }
                        return { ok: true, payload: [builtin] }
                    },
                    locale_list: () => {
                        localeLists += 1
                        if (!hasThemeCommands && localeLists === 1) {
                            return {
                                ok: false,
                                payload: {
                                    code: 'Localized',
                                    message: { kind: 'Forbidden', key: 'synthetic.catalog', args: {}, fallback: 'Synthetic locale refusal' },
                                },
                            }
                        }
                        return { ok: true, payload: [{ id: 'en', name: 'English', builtin: true }] }
                    },
                }
                const { ok, payload } = catalogReplies[request.command]()
                const response = JSON.stringify({ t: 'resp', seq: request.seq, ok, payload })
                if (isCatalogHeld) deferredCatalog = [...deferredCatalog, () => socket.send(response)]
                else socket.send(response)
                return
            }
            if (request.command === 'theme_get') {
                expect(hasThemeCommands).toBe(true)
                const args = z
                    .object({ themeId: z.enum(['taide-dark', 'synthetic']) })
                    .strict()
                    .parse(request.args)
                if (!catalogTheme) throw new Error('Synthetic base theme is missing')
                if (args.themeId === 'taide-dark') {
                    reply(catalogTheme)
                    return
                }
                if (!storedTheme) throw new Error('Synthetic custom theme is missing')
                reply({
                    ...catalogTheme,
                    id: storedTheme.id,
                    name: storedTheme.name,
                    colors: { ...catalogTheme.colors, ...storedTheme.colors },
                    syntax: { ...catalogTheme.syntax, ...storedTheme.syntax },
                    terminal: { ...catalogTheme.terminal, ...storedTheme.terminal },
                })
                return
            }
            if (request.command === 'theme_save' || request.command === 'theme_delete') {
                expect(hasThemeCommands).toBe(true)
                const editor = z
                    .object({
                        projectId: z.literal(PROJECT),
                        paneId: z.literal('synthetic-pane'),
                        tabId: z.literal('synthetic-settings'),
                        sourceThemeId: z.enum(['taide-dark', 'synthetic']),
                        isCreate: z.boolean(),
                    })
                    .strict()
                const mutation = z
                    .discriminatedUnion('operation', [
                        z.object({ operation: z.literal('save'), theme: storedThemeSchema, editor }).strict(),
                        z.object({ operation: z.literal('delete'), themeId: z.literal('synthetic'), editor }).strict(),
                    ])
                    .parse({ ...request.args, operation: request.command === 'theme_save' ? 'save' : 'delete' })
                if (mutation.operation === 'save') themeSaves += 1
                else themeDeletes += 1
                const finish = () => {
                    if (shouldRejectTheme) {
                        shouldRejectTheme = false
                        socket.send(
                            JSON.stringify({
                                t: 'resp',
                                seq: request.seq,
                                ok: false,
                                payload: { code: 'InvalidArgument', message: 'Synthetic theme save refusal' },
                            }),
                        )
                        return
                    }
                    if (mutation.operation === 'save') {
                        if (mutation.editor.isCreate) {
                            expect(storedTheme).toBeNull()
                            expect(mutation.editor.sourceThemeId).toBe('taide-dark')
                        } else {
                            expect(storedTheme?.id).toBe(mutation.editor.sourceThemeId)
                            expect(mutation.theme.id).toBe(mutation.editor.sourceThemeId)
                        }
                        storedTheme = mutation.theme
                        reply({ id: storedTheme.id, name: storedTheme.name, type: storedTheme.type, builtin: false })
                    } else {
                        expect(mutation.editor.isCreate).toBe(false)
                        expect(mutation.editor.sourceThemeId).toBe(mutation.themeId)
                        expect(storedTheme?.id).toBe(mutation.themeId)
                        storedTheme = null
                        reply(null)
                    }
                    socket.send(JSON.stringify({ t: 'event', event: 'theme:changed', payload: '{}' }))
                }
                if (isThemeHeld) deferredThemes = [...deferredThemes, finish]
                else finish()
                return
            }
            if (request.command === 'settings_set_theme') {
                expect(isPreferences).toBe(true)
                const args = z
                    .object({ themeId: z.enum(['synthetic-next', 'synthetic-failed']) })
                    .strict()
                    .parse(request.args)
                if (args.themeId === 'synthetic-failed') {
                    preferenceReply(JSON.stringify({ t: 'resp', seq: request.seq, ok: false, payload: { code: 'SYNTHETIC_SETTINGS' } }))
                    return
                }
                settings = { ...settings, themeId: args.themeId }
                preferenceReply(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: settings }))
                return
            }
            if (request.command === 'settings_update') {
                expect(isPreferences || isSettingsPreferences || isSettingsCode || isSettingsFontVisual || isSettingsKeybindings).toBe(true)
                const args = z
                    .object({ patch: z.record(z.string(), z.unknown()) })
                    .strict()
                    .parse(request.args)
                const fields = Object.fromEntries(Object.entries(args.patch).filter(([, value]) => value !== null))
                if (isSettingsKeybindings) {
                    const patch = z.object({ keymapOverrides: z.string() }).strict().parse(fields)
                    z.array(z.object({ actionId: z.string(), key: z.string(), mods: z.array(z.string()) }).passthrough()).parse(
                        JSON.parse(patch.keymapOverrides),
                    )
                    const finish = () => {
                        if (shouldRejectPreference) {
                            shouldRejectPreference = false
                            socket.send(
                                JSON.stringify({
                                    t: 'resp',
                                    seq: request.seq,
                                    ok: false,
                                    payload: { code: 'InvalidArgument', message: 'Synthetic keybinding refusal' },
                                }),
                            )
                            return
                        }
                        settings = settingsSchema.parse({ ...settings, ...patch })
                        reply(settings)
                    }
                    if (isPreferenceHeld) deferredPreferences = [...deferredPreferences, finish]
                    else finish()
                    return
                }
                if (isSettingsCode || isSettingsFontVisual) {
                    const patch = z
                        .object({
                            formatOnSave: z.boolean().optional(),
                            editorFontFamily: z.literal('Synthetic Mono').optional(),
                            editorFontSize: z.literal(CODE_FONT_SIZE).optional(),
                            shellOverride: z.literal('/synthetic/other').optional(),
                            terminalCursorStyle: z.literal('underline').optional(),
                        })
                        .strict()
                        .parse(fields)
                    expect(Object.keys(patch)).toHaveLength(1)
                    const finish = () => {
                        settings = settingsSchema.parse({ ...settings, ...patch })
                        reply(settings)
                    }
                    if (isPreferenceHeld) deferredPreferences = [...deferredPreferences, finish]
                    else finish()
                    return
                }
                if (isSettingsPreferences) {
                    const patch = z.object({ followSystemTheme: z.boolean() }).strict().parse(fields)
                    const nextSettings = { ...settings, ...patch }
                    const finish = () => {
                        if (shouldRejectPreference) {
                            shouldRejectPreference = false
                            socket.send(
                                JSON.stringify({
                                    t: 'resp',
                                    seq: request.seq,
                                    ok: false,
                                    payload: { code: 'InvalidArgument', message: 'Synthetic preference refusal' },
                                }),
                            )
                            return
                        }
                        settings = nextSettings
                        reply(settings)
                    }
                    if (isPreferenceHeld) deferredPreferences = [...deferredPreferences, finish]
                    else finish()
                    return
                }
                const patch = z
                    .object({ showSystemUsage: z.boolean().optional(), language: z.literal('ja').optional() })
                    .strict()
                    .parse(fields)
                expect(Object.keys(patch)).toHaveLength(1)
                settings = { ...settings, ...patch }
                preferenceReply(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: settings }))
                return
            }
            if (request.command === 'layout_get') {
                expect(request.args).toEqual({ projectId: PROJECT })
                if (isSettingsAppFile) {
                    reply(settingsLayout())
                    return
                }
                reply({
                    version: LAYOUT_SCHEMA_VERSION,
                    revision: 0,
                    root: isSettingsCatalog
                        ? {
                              node: 'leaf',
                              id: 'synthetic-pane',
                              tabs: [
                                  {
                                      id: 'synthetic-settings',
                                      kind: { kind: 'settings' },
                                      title: 'Settings',
                                      pinned: false,
                                      preview: false,
                                      dirty: false,
                                  },
                              ],
                              active: 'synthetic-settings',
                          }
                        : { node: 'leaf', id: 'first', tabs: [], active: null },
                    focusedPane: isSettingsCatalog ? 'synthetic-pane' : 'first',
                })
                return
            }
            if (request.command === 'file_list_mirrors') {
                expect(request.args).toEqual({ projectId: PROJECT })
                mirrorReads += 1
                if (isMirrorWrite || isApplication) {
                    socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: currentMirror ? [currentMirror.entry] : [] }))
                    return
                }
                let content = LATEST_MIRROR_TEXT
                if (mirrorReads === 1) content = RESTORED_TEXT
                if (mirrorReads === SHARED_VIEWS) content = 'obsolete draft'
                const response = JSON.stringify({
                    t: 'resp',
                    seq: request.seq,
                    ok: true,
                    payload: [{ path: PATH, content, savedAtMs: modified, diskModifiedMs: modified, conflict: true, sourceMissing: false }],
                })
                if (isMirrorHeld) {
                    deferredMirror = [...deferredMirror, () => socket.send(response)]
                    return
                }
                socket.send(response)
                return
            }
            if (request.command === 'layout_set_dirty') {
                const update = z
                    .object({ tabId: z.literal(isSettingsAppFile ? 'synthetic-app-file' : 'synthetic'), dirty: z.boolean() })
                    .strict()
                    .parse(request.args)
                if (isSettingsAppFile) {
                    isAppFileDirty = update.dirty
                    appFileRevision += 1
                }
                if (shouldFailDirty) {
                    shouldFailDirty = false
                    socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: false, payload: { code: 'SYNTHETIC_DIRTY' } }))
                    return
                }
                if (isDirtyHeld) {
                    deferredDirty = [...deferredDirty, () => reply(null)]
                    return
                }
                reply(null)
                return
            }
            if (request.command === 'file_mirror_dirty') {
                const args = z
                    .object({ projectId: z.literal(PROJECT), path: z.literal(PATH), content: z.string(), receipt: z.literal(true) })
                    .strict()
                    .parse(request.args)
                mirrorWrites += 1
                const commit = () => {
                    currentMirror = {
                        writeId: `mirrorwrite-synthetic-${mirrorWrites}`,
                        entry: {
                            path: args.path,
                            content: args.content,
                            savedAtMs: mirrorWrites,
                            diskModifiedMs: modified,
                            conflict: false,
                            sourceMissing: false,
                        },
                    }
                    reply(currentMirror)
                }
                if (isWriteHeld) {
                    deferredWrite = [...deferredWrite, commit]
                    return
                }
                commit()
                return
            }
            if (request.command === 'file_clear_mirror') {
                const args = z
                    .object({ projectId: z.literal(PROJECT), path: z.literal(PATH), expectedReceipt: receiptSchema })
                    .strict()
                    .parse(request.args)
                mirrorClears += 1
                expect(args.expectedReceipt).toEqual(currentMirror)
                currentMirror = null
                socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: true }))
                return
            }
            if (request.command === 'synthetic_cut_connection') {
                deferredPreferences = []
                isPreferenceHeld = false
                deferredSave = []
                isSaveHeld = false
                socket.close(RESTART_CODE, 'synthetic restart')
                return
            }
            if (request.command === 'file_open') {
                expect(request.args).toEqual({ path: PATH })
                const response = JSON.stringify({
                    t: 'resp',
                    seq: request.seq,
                    ok: true,
                    payload: {
                        path: PATH,
                        content: disk,
                        languageId: isPreparedConfig ? 'plaintext' : 'rust',
                        byteSize: disk.length,
                        lineCount: 1,
                        tier: 'normal',
                        readOnly: false,
                        encodingLossy: false,
                        modifiedMs: modified,
                        editorConfig: {
                            indentStyle: isPreparedConfig ? 'space' : null,
                            indentSize: isPreparedConfig ? FILE_INDENT_SIZE : null,
                            tabWidth: null,
                            trimTrailingWhitespace: isPreparedConfig ? shouldCleanFile : null,
                            insertFinalNewline: isPreparedConfig ? shouldCleanFile : null,
                        },
                    },
                })
                if (isReadHeld) {
                    deferredRead = [...deferredRead, () => socket.send(response)]
                    return
                }
                socket.send(response)
                return
            }
            if (request.command === 'file_save') {
                const args = z
                    .object({ path: z.literal(PATH), content: z.string() })
                    .strict()
                    .parse(request.args)
                const commit = () => {
                    disk = args.content
                    modified += 1
                    if (isMirrorWrite || isApplication) currentMirror = null
                    reply(null)
                }
                if (isSaveHeld) {
                    deferredSave = [...deferredSave, commit]
                    return
                }
                commit()
                return
            }
            if (request.command === 'theme_get_current') {
                const args = z
                    .object({ systemTheme: z.enum(['dark', 'light']) })
                    .strict()
                    .parse(request.args)
                if (isSettingsCatalog) {
                    if (!catalogTheme) throw new Error('Synthetic Settings theme is missing')
                    reply(catalogTheme)
                    return
                }
                reply({
                    id: 'synthetic',
                    name: 'Synthetic',
                    type: args.systemTheme,
                    colors: Object.fromEntries(
                        colorKeys.map((key) => {
                            if (!isCanvasVisual) return [key, '#102030']
                            if (key === 'editor.foreground' || key === 'app.foreground' || key === 'editor.cursor') return [key, '#f4f4f4']
                            if (key === 'editor.lineNumber') return [key, '#8896a4']
                            if (key === 'editor.selection') return [key, '#376074']
                            return [key, '#102030']
                        }),
                    ),
                    syntax: {},
                    terminal: {},
                })
                return
            }
            if (request.command === 'locale_get_current') {
                expect(request.args).toEqual({ systemLanguage: 'ko-KR' })
                reply({
                    id: isPreferences ? settings.language : 'ko',
                    name: 'Synthetic',
                    messages: {
                        'editor.changedOnDisk': 'changed on disk',
                        'editor.keepMine': 'keep mine',
                        'editor.viewDiskContent': 'view disk',
                        'editor.mirrorRestoredConflict': 'restored conflict',
                        'editor.mirrorRestored': 'restored draft',
                    },
                })
                return
            }
            const payloads = {
                project_list: isMirrorRestore || isMirrorWrite || isApplication ? [{ id: PROJECT, root: 'C:\\synthetic', name: 'Synthetic' }] : [],
                project_group_list: [],
                session_get_shell_state: { tree: null, focused: null, windowChrome: { zen: false, sidebarRailCollapsed: false } },
                settings_get: settings,
            }
            const command = z.enum(['project_list', 'project_group_list', 'session_get_shell_state', 'settings_get']).parse(request.command)
            expect(request.args).toBeNull()
            socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: payloads[command] }))
        },
    },
})
const verify = async () => {
    try {
        const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--use-mock-keychain'] })
        try {
            const context = await browser.newContext({
                locale: 'ko-KR',
                colorScheme: 'light',
                serviceWorkers: 'block',
                acceptDownloads: false,
                ...(isSettingsCode ||
                isSettingsFontVisual ||
                isSettingsCodeStyle ||
                isSettingsPickerLifecycle ||
                isSettingsPopupPresence ||
                isSettingsKeybindings ||
                isSettingsSnippets
                    ? { viewport: CODE_VIEWPORT }
                    : {}),
            })
            try {
                const page = await context.newPage()
                if (isSettingsFontVisual) {
                    await page.addInitScript(() => {
                        const CHANNEL = 'font-rasters'
                        const HASH_SEED = 2166136261
                        const HASH_MULTIPLIER = 16777619
                        const ALPHA_INDEX = 3
                        const PIXEL_BYTES = 4
                        let rasters: { font: string; hash: number; opaque: number; width: number; height: number }[] = []
                        CanvasRenderingContext2D.prototype.getImageData = new Proxy(CanvasRenderingContext2D.prototype.getImageData, {
                            apply: (target, receiver: unknown, args: unknown[]) => {
                                if (!(receiver instanceof CanvasRenderingContext2D) || args.some((value) => typeof value !== 'number')) {
                                    throw new Error('Invalid synthetic font pixel inspection')
                                }
                                const result: unknown = Reflect.apply(target, receiver, args)
                                if (!(result instanceof ImageData)) throw new Error('Missing font preview pixels')
                                const hash = result.data.reduce((current, value) => Math.imul(current ^ value, HASH_MULTIPLIER) >>> 0, HASH_SEED)
                                const opaque = result.data.reduce(
                                    (count, value, index) => count + Number(index % PIXEL_BYTES === ALPHA_INDEX && value > 0),
                                    0,
                                )
                                rasters = [...rasters, { font: receiver.font, hash, opaque, width: result.width, height: result.height }]
                                let channel = document.getElementById(CHANNEL)
                                if (!channel) {
                                    channel = document.createElement('pre')
                                    channel.id = CHANNEL
                                    channel.hidden = true
                                    document.body.append(channel)
                                }
                                channel.textContent = JSON.stringify(rasters)
                                return result
                            },
                        })
                    })
                }
                let errors = Array<string>()
                page.on('pageerror', (error) => {
                    errors = [...errors, error.message]
                })
                const state = async () => stateSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                const click = async (name: string) => {
                    await page.getByRole('button', { name, exact: true }).click()
                }
                const control = async (command: string) => {
                    const response = await context.request.post(`${server.url.origin}/__fixture/control`, { data: JSON.stringify(command) })
                    expect(response.ok()).toBe(true)
                }
                await page.goto(isSettingsAppFile ? `${server.url.href}?app-files` : server.url.href)
                if (isSettingsCatalog) {
                    await expect(page.locator('#state')).toHaveAttribute('data-ready', 'true', { timeout: DEADLINE_MS })
                    const catalogSchema = z.object({
                        frames: z.number().int().nonnegative(),
                        pumps: z.number().int().nonnegative(),
                        mount: z.number().int().positive().nullable(),
                        generation: z.number().int().positive().nullable(),
                        themes: z.array(z.string()),
                        locales: z.array(z.string()),
                        catalogErrors: z.array(z.string()),
                        failures: z.array(z.string()),
                        panicked: z.boolean(),
                        close: z.string(),
                    })
                    const catalogState = async () => {
                        await page.locator('#state').dispatchEvent('snapshot')
                        return catalogSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                    }
                    if (isSettingsSnippets) {
                        const snippetStateSchema = catalogSchema.extend({
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            code: z.object({ hitTargets: z.array(z.string()) }).nullable(),
                            focused: z.array(z.string()),
                            toasts: z.array(z.object({ title: z.string(), description: z.string().nullable() })),
                            snippets: z
                                .object({
                                    pending: z.boolean(),
                                    catalog: z.array(snippetFileSchema),
                                    states: z.array(
                                        z.object({
                                            files: z.array(snippetFileSchema),
                                            selected: z.string().nullable(),
                                            dirty: z.boolean(),
                                            drafts: z
                                                .array(z.object({ name: z.string(), prefix: z.string(), body: z.string(), scope: z.string() }))
                                                .nullable(),
                                            newOpen: z.boolean(),
                                            option: z.string(),
                                            globalName: z.string(),
                                            deleteOpen: z.boolean(),
                                            discard: z.boolean(),
                                        }),
                                    ),
                                })
                                .nullable(),
                        })
                        const snippetState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return snippetStateSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const widgetClick = async (label: string) => {
                            await expect
                                .poll(
                                    async () => {
                                        const current = await snippetState()
                                        const point = current.targets[label]
                                        const canvas = await page.locator('#canvas').boundingBox()
                                        if (
                                            !point ||
                                            !canvas ||
                                            point[0] < 0 ||
                                            point[1] < 0 ||
                                            point[0] >= canvas.width ||
                                            point[1] >= canvas.height
                                        )
                                            return false
                                        await page.mouse.move(canvas.x + point[0], canvas.y + point[1])
                                        return (await snippetState()).code?.hitTargets.includes(label) ?? false
                                    },
                                    { timeout: DEADLINE_MS },
                                )
                                .toBe(true)
                            const point = (await snippetState()).targets[label]
                            if (!point) throw new Error(`Missing actual snippet control: ${label}`)
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        const fill = async (label: string, value: string) => {
                            await widgetClick(label)
                            await page.keyboard.press('Meta+A')
                            await page.keyboard.insertText(value)
                        }
                        const editorState = async () => (await snippetState()).snippets?.states[0]
                        try {
                            await expect.poll(async () => (await snippetState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            await expect
                                .poll(() => requests.filter((request) => request.command === 'snippet_list').length, { timeout: DEADLINE_MS })
                                .toBe(1)
                            await widgetClick('settings.snippetsSectionTitle')
                            await widgetClick('settings.snippetsManage')
                            await expect.poll(async () => (await editorState())?.files, { timeout: DEADLINE_MS }).toEqual([])
                            await widgetClick('new-file')
                            await expect.poll(async () => (await editorState())?.newOpen, { timeout: DEADLINE_MS }).toBe(true)
                            await widgetClick('dialog-confirm')
                            await expect.poll(async () => (await editorState())?.selected, { timeout: DEADLINE_MS }).toBe('rust.json')
                            await widgetClick('add-entry')
                            await fill('name', 'Synthetic')
                            await fill('prefix', 'syn')
                            await fill('body', SNIPPET_BODY)
                            await expect.poll(async () => (await editorState())?.drafts?.[0]?.body, { timeout: DEADLINE_MS }).toBe(SNIPPET_BODY)
                            await control('hold-snippet')
                            await widgetClick('save')
                            await expect.poll(async () => (await snippetState()).snippets?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await snippetState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('reject-snippet')
                            await expect.poll(async () => (await snippetState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Snippet')
                            await expect
                                .poll(async () => (await snippetState()).toasts.some((toast) => toast.title === 'snippetEditor.parseError'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            const failed = await snippetState()
                            expect(failed.snippets?.states[0]?.dirty).toBe(true)
                            expect(failed.snippets?.states[0]?.drafts?.[0]?.body).toBe(SNIPPET_BODY)
                            expect(failed.toasts.some((toast) => toast.title === 'snippetEditor.parseError')).toBe(true)
                            expect(snippetFiles[0]?.snippets).toEqual({})
                            await click('Cancel close')
                            await expect.poll(async () => (await snippetState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                            await widgetClick('save')
                            await expect.poll(async () => (await editorState())?.dirty, { timeout: DEADLINE_MS }).toBe(false)
                            await expect
                                .poll(async () => (await snippetState()).snippets?.catalog[0]?.snippets.Synthetic?.body, { timeout: DEADLINE_MS })
                                .toBe(SNIPPET_BODY)
                            const saved = await snippetState()
                            const cachedReads = requests.filter((request) => request.command === 'snippet_list').length
                            await widgetClick('back')
                            await expect.poll(async () => (await snippetState()).snippets?.states.length, { timeout: DEADLINE_MS }).toBe(0)
                            await widgetClick('settings.snippetsSectionTitle')
                            await widgetClick('settings.snippetsManage')
                            await widgetClick('file:rust.json')
                            await expect.poll(async () => (await editorState())?.drafts?.[0]?.body, { timeout: DEADLINE_MS }).toBe(SNIPPET_BODY)
                            expect(requests.filter((request) => request.command === 'snippet_list')).toHaveLength(cachedReads)
                            await widgetClick('new-file')
                            await widgetClick('snippetEditor.newFileLanguagePlaceholder')
                            await expect.poll(async () => (await snippetState()).focused, { timeout: DEADLINE_MS }).toContain('picker-dialog')
                            await page.keyboard.press('Home')
                            await expect.poll(async () => (await snippetState()).focused, { timeout: DEADLINE_MS }).toContain('picker-dialog')
                            await page.keyboard.press('Enter')
                            await expect.poll(async () => (await snippetState()).focused, { timeout: DEADLINE_MS }).toContain('new-global-name')
                            await page.keyboard.insertText('global-synthetic')
                            await expect.poll(async () => (await editorState())?.globalName, { timeout: DEADLINE_MS }).toBe('global-synthetic')
                            await widgetClick('dialog-confirm')
                            await expect
                                .poll(async () => (await editorState())?.selected, { timeout: DEADLINE_MS })
                                .toBe('global-synthetic.code-snippets')
                            await widgetClick('delete-file')
                            await expect.poll(async () => (await snippetState()).focused, { timeout: DEADLINE_MS }).toContain('dialog-cancel')
                            await widgetClick('dialog-confirm')
                            await expect
                                .poll(async () => (await editorState())?.files.map((file) => file.fileName), { timeout: DEADLINE_MS })
                                .toEqual(['rust.json'])
                            await widgetClick('file:rust.json')
                            await fill('prefix', 'discarded')
                            await widgetClick('back')
                            await expect.poll(async () => (await editorState())?.discard, { timeout: DEADLINE_MS }).toBe(true)
                            await widgetClick('dialog-cancel')
                            await expect.poll(async () => (await editorState())?.discard, { timeout: DEADLINE_MS }).toBe(false)
                            expect((await editorState())?.drafts?.[0]?.prefix).toBe('discarded')
                            await widgetClick('back')
                            await widgetClick('dialog-confirm')
                            await expect.poll(async () => (await snippetState()).snippets?.states.length, { timeout: DEADLINE_MS }).toBe(0)
                            await widgetClick('settings.snippetsSectionTitle')
                            await widgetClick('settings.snippetsManage')
                            await widgetClick('file:rust.json')
                            await expect.poll(async () => (await editorState())?.drafts?.[0]?.prefix, { timeout: DEADLINE_MS }).toBe('syn')
                            await fill('body', 'fn revised() {}')
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-snippets.png') })
                            await control('hold-snippet')
                            await widgetClick('save')
                            await expect.poll(async () => (await snippetState()).snippets?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Unmount Settings')
                            await expect.poll(async () => (await snippetState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                            await click('Close Settings')
                            await expect.poll(async () => (await snippetState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('release-snippet')
                            await expect.poll(async () => (await snippetState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(snippetFiles).toHaveLength(1)
                            expect(snippetFiles[0]?.snippets.Synthetic?.body).toBe('fn revised() {}')
                            expect(requests.filter((request) => request.command === 'snippet_save')).toHaveLength(SNIPPET_SAVES)
                            expect(requests.filter((request) => request.command === 'snippet_delete')).toHaveLength(1)
                            expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                            const closed = await snippetState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await snippetState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, failed, saved, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-snippets-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-snippets-failure.json'),
                                JSON.stringify({ state: await snippetState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-snippets-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsKeybindings) {
                        const keybindingSchema = catalogSchema.extend({
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            code: z.object({ hitTargets: z.array(z.string()) }).nullable(),
                            preferences: z.object({ pending: z.boolean(), results: z.array(z.string()) }).nullable(),
                            keybindings: z
                                .object({
                                    open: z.boolean(),
                                    capturing: z.boolean(),
                                    query: z.string(),
                                    focused: z.string().nullable(),
                                    targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                                    hitTargets: z.array(z.string()),
                                    overrides: z.string().nullable(),
                                })
                                .nullable(),
                            toasts: z.array(z.object({ title: z.string(), description: z.string().nullable() })),
                        })
                        const keybindingState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return keybindingSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const widgetClick = async (label: string, modal = false) => {
                            await expect
                                .poll(
                                    async () => {
                                        const current = await keybindingState()
                                        const point = (modal ? current.keybindings?.targets : current.targets)?.[label]
                                        const canvas = await page.locator('#canvas').boundingBox()
                                        if (
                                            !point ||
                                            !canvas ||
                                            point[0] < 0 ||
                                            point[1] < 0 ||
                                            point[0] >= canvas.width ||
                                            point[1] >= canvas.height
                                        )
                                            return false
                                        await page.mouse.move(canvas.x + point[0], canvas.y + point[1])
                                        const after = await keybindingState()
                                        return (modal ? after.keybindings?.hitTargets : after.code?.hitTargets)?.includes(label) ?? false
                                    },
                                    { timeout: DEADLINE_MS },
                                )
                                .toBe(true)
                            const current = await keybindingState()
                            const point = (modal ? current.keybindings?.targets : current.targets)?.[label]
                            if (!point) throw new Error(`Missing actual keybinding control: ${label}`)
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        try {
                            await expect.poll(async () => (await keybindingState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            await widgetClick('settings.keymap')
                            await expect
                                .poll(async () => Object.hasOwn((await keybindingState()).targets, 'settings.keymapOpenEditor'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            await widgetClick('settings.keymapOpenEditor')
                            await expect.poll(async () => (await keybindingState()).keybindings?.focused, { timeout: DEADLINE_MS }).toBe('search')
                            await page.keyboard.insertText('quick-open')
                            await expect.poll(async () => (await keybindingState()).keybindings?.query, { timeout: DEADLINE_MS }).toBe('quick-open')
                            await expect
                                .poll(async () => Object.hasOwn((await keybindingState()).keybindings?.targets ?? {}, 'row:quick-open:Change'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            await widgetClick('row:quick-open:Change', true)
                            await expect.poll(async () => (await keybindingState()).keybindings?.capturing, { timeout: DEADLINE_MS }).toBe(true)
                            await page.keyboard.press('Meta+Shift+Y')
                            await expect
                                .poll(async () => Object.hasOwn((await keybindingState()).keybindings?.targets ?? {}, 'row:quick-open:Confirm'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            await widgetClick('row:quick-open:Confirm', true)
                            await expect.poll(async () => (await keybindingState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(false)
                            await expect
                                .poll(async () => JSON.parse((await keybindingState()).keybindings?.overrides ?? '[]'), { timeout: DEADLINE_MS })
                                .toEqual([{ actionId: 'quick-open', key: 'y', mods: ['mod', 'shift'] }])
                            const assigned = await keybindingState()
                            expect(assigned.keybindings?.query).toBe('quick-open')
                            expect(assigned.keybindings?.capturing).toBe(false)
                            await expect
                                .poll(async () => Object.hasOwn((await keybindingState()).keybindings?.targets ?? {}, 'row:quick-open:Reset'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            await widgetClick('row:quick-open:Reset', true)
                            await expect.poll(async () => (await keybindingState()).keybindings?.overrides, { timeout: DEADLINE_MS }).toBe('[]')
                            await control('hold-preference')
                            await widgetClick('row:quick-open:Unbind', true)
                            await expect.poll(async () => (await keybindingState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await keybindingState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('reject-preference')
                            await expect.poll(async () => (await keybindingState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Preference')
                            const failed = await keybindingState()
                            expect(failed.keybindings?.overrides).toBe('[]')
                            expect(failed.toasts.some((toast) => toast.description === 'Synthetic keybinding refusal')).toBe(true)
                            expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(KEYBINDING_WRITES - 1)
                            await click('Cancel close')
                            await expect.poll(async () => (await keybindingState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                            await expect
                                .poll(async () => Object.hasOwn((await keybindingState()).keybindings?.targets ?? {}, 'row:quick-open:Unbind'), {
                                    timeout: DEADLINE_MS,
                                })
                                .toBe(true)
                            await control('hold-preference')
                            await widgetClick('row:quick-open:Unbind', true)
                            await expect.poll(async () => (await keybindingState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await keybindingState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('release-preference')
                            await expect.poll(async () => (await keybindingState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(JSON.parse(z.string().parse(settings?.keymapOverrides))).toEqual([{ actionId: 'quick-open', key: '', mods: [] }])
                            const closed = await keybindingState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(KEYBINDING_WRITES)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await keybindingState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, assigned, failed, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-keybindings-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-keybindings-failure.json'),
                                JSON.stringify({ state: await keybindingState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-keybindings-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsResourceCache) {
                        const cacheSchema = catalogSchema.extend({
                            code: z
                                .object({
                                    resources: z.array(
                                        z.object({
                                            fonts: z.array(z.object({ name: z.string(), monospaced: z.boolean() })).nullable(),
                                            shells: z.array(z.object({ id: z.string(), path: z.string() })).nullable(),
                                        }),
                                    ),
                                })
                                .nullable(),
                        })
                        const cacheState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return cacheSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        try {
                            await expect.poll(async () => (await cacheState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            await expect.poll(() => resourceCounts, { timeout: DEADLINE_MS }).toEqual({ font_list: 1, shell_profiles: 1 })
                            const pending = await cacheState()
                            expect(pending.code?.resources).toEqual([{ fonts: null, shells: null }])
                            await click('Unmount Settings')
                            await expect.poll(async () => (await cacheState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                            await control('release-resources')
                            await expect.poll(async () => (await cacheState()).pumps, { timeout: DEADLINE_MS }).toBeGreaterThan(pending.pumps)
                            await click('Mount Settings')
                            await expect
                                .poll(async () => (await cacheState()).code?.resources[0]?.fonts?.map((font) => font.name), { timeout: DEADLINE_MS })
                                .toEqual(['Synthetic Mono', 'Synthetic Sans'])
                            await expect
                                .poll(async () => (await cacheState()).code?.resources[0]?.shells?.map((shell) => shell.id), { timeout: DEADLINE_MS })
                                .toEqual(['synthetic-shell', 'synthetic-other'])
                            expect(resourceCounts).toEqual({ font_list: 1, shell_profiles: SHARED_VIEWS })
                            await click('Close Settings')
                            await expect.poll(async () => (await cacheState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(requests.some((request) => request.command === 'settings_update')).toBe(false)
                            const closed = await cacheState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await cacheState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, pending, closed, resourceCounts, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-resource-cache-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-resource-cache-failure.json'),
                                JSON.stringify({ state: await cacheState(), resourceCounts, requests, errors }, null, 2),
                            )
                            throw error
                        }
                        return
                    }
                    if (isSettingsCode || isSettingsFontVisual || isSettingsCodeStyle || isSettingsPickerLifecycle || isSettingsPopupPresence) {
                        const codeSchema = catalogSchema.extend({
                            focused: z.array(z.string()),
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            preferences: z.object({ pending: z.boolean(), results: z.array(z.string()) }).nullable(),
                            code: z
                                .object({
                                    hitTargets: z.array(z.string()),
                                    popups: z.array(
                                        z.object({
                                            field: z.string(),
                                            open: z.boolean(),
                                            opacity: z.number(),
                                            scale: z.number(),
                                            active: z.boolean(),
                                        }),
                                    ),
                                    settings: z
                                        .object({
                                            formatOnSave: z.boolean(),
                                            editorFontFamily: z.string().nullable(),
                                            editorFontSize: z.number(),
                                            shellOverride: z.string().nullable(),
                                            terminalCursorStyle: z.string(),
                                        })
                                        .nullable(),
                                    resources: z.array(
                                        z.object({
                                            fonts: z.array(z.object({ name: z.string(), monospaced: z.boolean() })).nullable(),
                                            shells: z.array(z.object({ id: z.string(), path: z.string() })).nullable(),
                                        }),
                                    ),
                                })
                                .nullable(),
                        })
                        const codeState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return codeSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const widgetClick = async (key: string) => {
                            await expect.poll(async () => Object.hasOwn((await codeState()).targets, key), { timeout: DEADLINE_MS }).toBe(true)
                            const box = await page.locator('#canvas').boundingBox()
                            if (!box) throw new Error('Missing actual canvas')
                            const initial = (await codeState()).targets[key]
                            if (!initial) throw new Error(`Missing original widget ${key}`)
                            if (initial[1] < 0 || initial[1] > box.height) {
                                await page.mouse.move(box.x + initial[0], box.y + box.height / 2)
                                await page.mouse.wheel(0, initial[1] - box.height / 2)
                            }
                            await expect
                                .poll(
                                    async () => {
                                        const point = (await codeState()).targets[key]
                                        if (!point || point[1] < 0 || point[1] > box.height) return false
                                        await page.mouse.move(box.x + point[0], box.y + point[1])
                                        return (await codeState()).code?.hitTargets.includes(key) ?? false
                                    },
                                    { timeout: DEADLINE_MS },
                                )
                                .toBe(true)
                            const point = (await codeState()).targets[key]
                            if (!point) throw new Error(`Original widget disappeared: ${key}`)
                            await page.mouse.click(box.x + point[0], box.y + point[1])
                        }
                        if (isSettingsPopupPresence) {
                            try {
                                await expect
                                    .poll(async () => (await codeState()).code?.resources[0]?.fonts?.length, { timeout: DEADLINE_MS })
                                    .toBe(SHARED_VIEWS)
                                await widgetClick('settings.editor')
                                await widgetClick('settings.editorFontFamily')
                                await expect
                                    .poll(async () => (await codeState()).code?.popups.some((popup) => popup.open && !popup.active), {
                                        timeout: DEADLINE_MS,
                                    })
                                    .toBe(true)
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                                for (const key of ['Tab', 'Shift+Tab']) {
                                    const before = await codeState()
                                    await page.keyboard.press(key)
                                    await expect.poll(async () => (await codeState()).frames, { timeout: DEADLINE_MS }).toBeGreaterThan(before.frames)
                                    expect((await codeState()).focused).toContain('font-search')
                                }
                                await page.keyboard.press('Escape')
                                await expect
                                    .poll(
                                        async () =>
                                            (await codeState()).code?.popups.some((popup) => !popup.open && popup.active && popup.opacity > 0),
                                        { timeout: DEADLINE_MS, intervals: [POPUP_POLL_MS] },
                                    )
                                    .toBe(true)
                                const fontClosing = await codeState()
                                expect(Object.hasOwn(fontClosing.targets, 'font-search')).toBe(true)
                                expect(fontClosing.focused).toContain('font-search')
                                await expect.poll(async () => (await codeState()).code?.popups, { timeout: DEADLINE_MS }).toEqual([])
                                await expect
                                    .poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS })
                                    .toContain('settings.editorFontFamily')
                                await widgetClick('settings.editorRenderWhitespace')
                                await expect
                                    .poll(async () => (await codeState()).code?.popups.some((popup) => popup.open && !popup.active), {
                                        timeout: DEADLINE_MS,
                                    })
                                    .toBe(true)
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('picker-dialog')
                                for (const key of ['Tab', 'Shift+Tab']) {
                                    const before = await codeState()
                                    await page.keyboard.press(key)
                                    await expect.poll(async () => (await codeState()).frames, { timeout: DEADLINE_MS }).toBeGreaterThan(before.frames)
                                    expect((await codeState()).focused).toContain('picker-dialog')
                                }
                                await page.keyboard.press('Escape')
                                await expect
                                    .poll(
                                        async () =>
                                            (await codeState()).code?.popups.some((popup) => !popup.open && popup.active && popup.opacity > 0),
                                        { timeout: DEADLINE_MS, intervals: [POPUP_POLL_MS] },
                                    )
                                    .toBe(true)
                                const optionClosing = await codeState()
                                expect(optionClosing.focused).toContain('picker-dialog')
                                await expect.poll(async () => (await codeState()).code?.popups, { timeout: DEADLINE_MS }).toEqual([])
                                await expect
                                    .poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS })
                                    .toContain('settings.editorRenderWhitespace')
                                await widgetClick('settings.editorFontFamily')
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                                await click('Unmount Settings')
                                await expect.poll(async () => (await codeState()).code?.popups, { timeout: DEADLINE_MS }).toEqual([])
                                const unmounted = await codeState()
                                expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(0)
                                await click('Close Settings')
                                await expect.poll(async () => (await codeState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                                const count = requests.length
                                await page.waitForTimeout(QUIET_MS)
                                expect(requests).toHaveLength(count)
                                expect(errors).toEqual([])
                                const result = { status: 'passed', mode, fontClosing, optionClosing, unmounted, active, requests, errors }
                                await Bun.write(resolve(fixture, 'settings-popup-presence-result.json'), JSON.stringify(result, null, 2))
                                console.info(JSON.stringify(result))
                            } catch (error) {
                                await Bun.write(
                                    resolve(fixture, 'settings-popup-presence-failure.json'),
                                    JSON.stringify(
                                        { state: JSON.parse((await page.locator('#state').textContent()) ?? '{}'), requests, errors },
                                        null,
                                        2,
                                    ),
                                )
                                throw error
                            }
                            return
                        }
                        if (isSettingsPickerLifecycle) {
                            try {
                                await expect
                                    .poll(async () => (await codeState()).code?.resources[0]?.fonts?.length, { timeout: DEADLINE_MS })
                                    .toBe(SHARED_VIEWS)
                                await widgetClick('settings.editor')
                                await widgetClick('settings.editorFontFamily')
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                                await page.keyboard.press('ArrowLeft')
                                await page.keyboard.type('xyz')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'picker-option'), { timeout: DEADLINE_MS })
                                    .toBe(false)
                                const empty = await codeState()
                                expect(empty.focused).toContain('font-search')
                                await page.keyboard.press('Escape')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'font-search'), { timeout: DEADLINE_MS })
                                    .toBe(false)
                                await expect
                                    .poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS })
                                    .toContain('settings.editorFontFamily')
                                await widgetClick('settings.editorRenderWhitespace')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'picker-option'), { timeout: DEADLINE_MS })
                                    .toBe(true)
                                await page.keyboard.press('End')
                                await page.keyboard.press('Escape')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'picker-option'), { timeout: DEADLINE_MS })
                                    .toBe(false)
                                await widgetClick('settings.editorFontFamily')
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                                await widgetClick('settings.editor')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'font-search'), { timeout: DEADLINE_MS })
                                    .toBe(false)
                                const dismissed = await codeState()
                                expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(0)
                                const count = requests.length
                                await click('Drop')
                                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                                await page.waitForTimeout(QUIET_MS)
                                expect(requests).toHaveLength(count)
                                expect(errors).toEqual([])
                                const result = { status: 'passed', mode, empty, dismissed, active, requests, errors }
                                await Bun.write(resolve(fixture, 'settings-picker-lifecycle-result.json'), JSON.stringify(result, null, 2))
                                console.info(JSON.stringify(result))
                            } catch (error) {
                                await Bun.write(
                                    resolve(fixture, 'settings-picker-lifecycle-failure.json'),
                                    JSON.stringify(
                                        {
                                            state: codeSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}')),
                                            requests,
                                            errors,
                                        },
                                        null,
                                        2,
                                    ),
                                )
                                throw error
                            }
                            return
                        }
                        if (isSettingsCodeStyle) {
                            try {
                                await expect
                                    .poll(async () => (await codeState()).code?.resources[0]?.fonts?.length, { timeout: DEADLINE_MS })
                                    .toBe(SHARED_VIEWS)
                                await widgetClick('settings.editor')
                                await widgetClick('settings.editorFontFamily')
                                await expect
                                    .poll(async () => Object.hasOwn((await codeState()).targets, 'font-search'), { timeout: DEADLINE_MS })
                                    .toBe(true)
                                const rendered = await codeState()
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-code-style.png') })
                                expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(0)
                                await click('Drop')
                                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                                expect(errors).toEqual([])
                                const result = { status: 'passed', mode, rendered, active, requests, errors }
                                await Bun.write(resolve(fixture, 'settings-code-style-result.json'), JSON.stringify(result, null, 2))
                                console.info(JSON.stringify(result))
                            } catch (error) {
                                await Bun.write(
                                    resolve(fixture, 'settings-code-style-failure.json'),
                                    JSON.stringify({ state: await codeState(), requests, errors }, null, 2),
                                )
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-code-style-failure.png') })
                                throw error
                            }
                            return
                        }
                        if (isSettingsFontVisual) {
                            const rastersSchema = z.array(
                                z.object({ font: z.string(), hash: z.number(), opaque: z.number(), width: z.number(), height: z.number() }),
                            )
                            const fontRasters = async () =>
                                rastersSchema.parse(JSON.parse((await page.locator('#font-rasters').textContent()) ?? '[]'))
                            try {
                                await expect
                                    .poll(async () => (await codeState()).code?.resources[0]?.fonts?.length, { timeout: DEADLINE_MS })
                                    .toBe(SHARED_VIEWS)
                                await widgetClick('settings.editor')
                                await widgetClick('editor-monospace-filter')
                                await widgetClick('settings.editorFontFamily')
                                await expect
                                    .poll(
                                        async () =>
                                            (await fontRasters()).some((raster) => raster.font.includes('Synthetic Sans') && raster.opaque > 0),
                                        { timeout: DEADLINE_MS },
                                    )
                                    .toBe(true)
                                const before = await fontRasters()
                                const monoBefore = before.findLast((raster) => raster.font.includes('Synthetic Mono'))
                                const sansBefore = before.findLast((raster) => raster.font.includes('Synthetic Sans'))
                                if (!monoBefore || !sansBefore) throw new Error('Missing original fallback font pixels')
                                await page.evaluate(async () => {
                                    document.fonts.add(new FontFace('Synthetic Mono', 'url(/mono.ttf)'))
                                    document.fonts.add(new FontFace('Synthetic Sans', 'url(/sans.ttf)'))
                                    await Promise.all([document.fonts.load('12px "Synthetic Mono"'), document.fonts.load('12px "Synthetic Sans"')])
                                    await document.fonts.ready
                                })
                                await expect
                                    .poll(async () => (await fontRasters()).findLast((raster) => raster.font.includes('Synthetic Mono'))?.hash, {
                                        timeout: DEADLINE_MS,
                                    })
                                    .not.toBe(monoBefore.hash)
                                await expect
                                    .poll(async () => (await fontRasters()).findLast((raster) => raster.font.includes('Synthetic Sans'))?.hash, {
                                        timeout: DEADLINE_MS,
                                    })
                                    .not.toBe(sansBefore.hash)
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-font-visual.png') })
                                await widgetClick('font-search')
                                await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                                await page.keyboard.type('Synthetic Mono')
                                await page.keyboard.press('Enter')
                                await expect
                                    .poll(async () => (await codeState()).code?.settings?.editorFontFamily, { timeout: DEADLINE_MS })
                                    .toBe('Synthetic Mono')
                                await expect.poll(async () => (await codeState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(false)
                                await expect
                                    .poll(async () => (await fontRasters()).filter((raster) => raster.font.includes('Synthetic Mono')).length, {
                                        timeout: DEADLINE_MS,
                                    })
                                    .toBeGreaterThan(before.filter((raster) => raster.font.includes('Synthetic Mono')).length)
                                const painted = await fontRasters()
                                expect(painted.every((raster) => raster.opaque > 0)).toBe(true)
                                await click('Close Settings')
                                await expect.poll(async () => (await codeState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                                const closed = await codeState()
                                const rasterCount = (await fontRasters()).length
                                const count = requests.length
                                await page.evaluate(() => document.fonts.dispatchEvent(new Event('loadingdone')))
                                await page.waitForTimeout(QUIET_MS)
                                expect(await codeState()).toEqual(closed)
                                expect(await fontRasters()).toHaveLength(rasterCount)
                                expect(requests).toHaveLength(count)
                                expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(1)
                                expect(errors).toEqual([])
                                expect(closed.failures).toEqual([])
                                expect(closed.panicked).toBe(false)
                                await click('Drop')
                                const result = { status: 'passed', mode, before, painted, closed, rasterCount, active, requests, errors }
                                await Bun.write(resolve(fixture, 'settings-font-visual-result.json'), JSON.stringify(result, null, 2))
                                console.info(JSON.stringify(result))
                            } catch (error) {
                                await Bun.write(
                                    resolve(fixture, 'settings-font-visual-failure.json'),
                                    JSON.stringify({ state: await codeState(), requests, errors }, null, 2),
                                )
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-font-visual-failure.png') })
                                throw error
                            }
                            return
                        }
                        try {
                            await expect
                                .poll(async () => (await codeState()).code?.resources[0]?.fonts?.map((font) => font.name), { timeout: DEADLINE_MS })
                                .toEqual(['Synthetic Mono', 'Synthetic Sans'])
                            await expect
                                .poll(async () => (await codeState()).code?.resources[0]?.shells?.map((shell) => shell.id), { timeout: DEADLINE_MS })
                                .toEqual(['synthetic-shell', 'synthetic-other'])
                            const original = (await codeState()).code?.settings?.formatOnSave
                            await widgetClick('settings.editor')
                            await widgetClick('settings.formatOnSave')
                            await expect.poll(async () => (await codeState()).code?.settings?.formatOnSave, { timeout: DEADLINE_MS }).toBe(!original)
                            await widgetClick('settings.editorFontFamily')
                            await expect
                                .poll(async () => Object.hasOwn((await codeState()).targets, 'font-search'), { timeout: DEADLINE_MS })
                                .toBe(true)
                            await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                            await widgetClick('font-search')
                            await expect.poll(async () => (await codeState()).focused, { timeout: DEADLINE_MS }).toContain('font-search')
                            await page.keyboard.type('sm')
                            await page.keyboard.press('Enter')
                            await expect
                                .poll(async () => (await codeState()).code?.settings?.editorFontFamily, { timeout: DEADLINE_MS })
                                .toBe('Synthetic Mono')
                            await widgetClick('settings.editorFontSize')
                            await page.keyboard.press('Meta+A')
                            await page.keyboard.insertText(String(CODE_FONT_SIZE))
                            await widgetClick('settings.terminal')
                            await expect
                                .poll(async () => (await codeState()).code?.settings?.editorFontSize, { timeout: DEADLINE_MS })
                                .toBe(CODE_FONT_SIZE)
                            await widgetClick('shell-profile')
                            await expect
                                .poll(async () => (await codeState()).code?.settings?.shellOverride, { timeout: DEADLINE_MS })
                                .toBe('/synthetic/other')
                            await control('hold-preference')
                            await widgetClick('settings.terminalCursorStyle')
                            await page.keyboard.press('End')
                            await page.keyboard.press('Enter')
                            await expect.poll(async () => (await codeState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-code.png') })
                            await click('Close Settings')
                            await expect.poll(async () => (await codeState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            expect(active).toBe(1)
                            await control('release-preference')
                            await expect.poll(async () => (await codeState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(settings?.terminalCursorStyle).toBe('underline')
                            expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(CODE_WRITES)
                            for (const command of ['font_list', 'shell_profiles'])
                                expect(requests.filter((request) => request.command === command)).toHaveLength(1)
                            const closed = await codeState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await codeState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-code-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-code-failure.json'),
                                JSON.stringify({ state: await codeState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-code-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsAppFile) {
                        const appFileSchema = catalogSchema.extend({
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            appFile: z.object({ text: z.string(), dirty: z.boolean(), pending: z.boolean() }).nullable(),
                            toasts: z.array(z.object({ title: z.string(), description: z.string().nullable() })),
                        })
                        const appFileState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return appFileSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const widgetClick = async (key: string) => {
                            await expect.poll(async () => Object.hasOwn((await appFileState()).targets, key), { timeout: DEADLINE_MS }).toBe(true)
                            const point = (await appFileState()).targets[key]
                            if (!point) throw new Error(`Missing original widget ${key}`)
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        try {
                            await expect.poll(async () => (await appFileState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            await widgetClick('open-settings-file')
                            await expect.poll(async () => (await appFileState()).appFile?.text, { timeout: DEADLINE_MS }).toBe(appFileContent)
                            await widgetClick('app-file-editor')
                            await page.keyboard.press('Meta+A')
                            const draft = JSON.stringify({ editorTabSize: FILE_INDENT_SIZE })
                            await page.keyboard.insertText(draft)
                            await expect.poll(async () => (await appFileState()).appFile?.dirty, { timeout: DEADLINE_MS }).toBe(true)
                            await control('hold-write')
                            await page.keyboard.press('Meta+S')
                            await expect.poll(async () => (await appFileState()).appFile?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await appFileState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('release-write')
                            await expect.poll(async () => (await appFileState()).close, { timeout: DEADLINE_MS }).toContain('Failed(AppFile(')
                            await expect
                                .poll(async () => (await appFileState()).toasts, { timeout: DEADLINE_MS })
                                .toEqual([{ title: 'settings.settingsJsonInvalid', description: null }])
                            const failed = await appFileState()
                            expect(failed.appFile?.text).toBe(draft)
                            expect(failed.appFile?.dirty).toBe(true)
                            expect(active).toBe(1)
                            await click('Cancel close')
                            await expect.poll(async () => (await appFileState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                            await control('hold-write')
                            await widgetClick('app-file-editor')
                            await page.keyboard.press('Meta+S')
                            await expect.poll(async () => (await appFileState()).appFile?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await appFileState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('release-write')
                            await expect.poll(async () => (await appFileState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(isAppFileDirty).toBe(false)
                            z.object({ editorTabSize: z.literal(FILE_INDENT_SIZE) })
                                .passthrough()
                                .parse(JSON.parse(appFileContent))
                            expect(requests.filter((request) => request.command === 'app_file_write')).toHaveLength(APP_FILE_WRITES)
                            expect(requests.filter((request) => request.command === 'app_file_read').length).toBeGreaterThan(1)
                            expect(
                                requests.some(
                                    (request) =>
                                        request.command === 'file_open' || request.command === 'file_save' || request.command.includes('mirror'),
                                ),
                            ).toBe(false)
                            const closed = await appFileState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await appFileState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, failed, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-app-file-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-app-file-failure.json'),
                                JSON.stringify({ state: await appFileState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-app-file-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsPreview) {
                        const previewSchema = catalogSchema.extend({
                            editing: z.string().nullable(),
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            displayTheme: z.object({ background: z.string().nullable(), other: z.string().nullable() }).nullable(),
                            focused: z.array(z.string()),
                            hexInput: z
                                .object({
                                    id: z.string(),
                                    focused: z.string(),
                                    enabled: z.boolean(),
                                    hovered: z.boolean(),
                                    containsPointer: z.boolean(),
                                    layer: z.string(),
                                    hitLayer: z.string(),
                                    input: z.object({ focused: z.boolean(), pointer: z.string() }),
                                })
                                .nullable(),
                        })
                        const previewState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return previewSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const widgetClick = async (key: string) => {
                            await expect.poll(async () => Object.hasOwn((await previewState()).targets, key), { timeout: DEADLINE_MS }).toBe(true)
                            const point = (await previewState()).targets[key]
                            if (!point) throw new Error(`Missing original widget ${key}`)
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        try {
                            await expect.poll(async () => (await previewState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            const baseline = (await previewState()).displayTheme
                            if (!baseline?.background || !baseline.other) throw new Error('Missing base theme')
                            await widgetClick('create-theme')
                            await expect.poll(async () => (await previewState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                            await widgetClick('search')
                            await page.keyboard.insertText('background')
                            await widgetClick('picker-Colors-app.background-0')
                            await expect
                                .poll(
                                    async () => {
                                        const snapshot = await previewState()
                                        const point = snapshot.targets['picker-Colors-app.background-3']
                                        const canvas = await page.locator('#canvas').boundingBox()
                                        if (!point || !canvas) return false
                                        await page.mouse.move(canvas.x + point[0], canvas.y + point[1])
                                        return (await previewState()).hexInput?.containsPointer ?? false
                                    },
                                    { timeout: DEADLINE_MS },
                                )
                                .toBe(true)
                            await widgetClick('picker-Colors-app.background-3')
                            await expect
                                .poll(async () => (await previewState()).focused, { timeout: DEADLINE_MS })
                                .toContain('picker-Colors-app.background-3')
                            await page.keyboard.press('Meta+A')
                            const background = '#244466'
                            await page.keyboard.insertText(background)
                            await widgetClick('name')
                            await expect
                                .poll(async () => (await previewState()).displayTheme, { timeout: DEADLINE_MS })
                                .toEqual({ background, other: baseline.other })
                            const previewed = await previewState()
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-preview.png') })
                            await click('Unmount Settings')
                            await expect.poll(async () => (await previewState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                            await expect.poll(async () => (await previewState()).displayTheme, { timeout: DEADLINE_MS }).toEqual(baseline)
                            expect(themeSaves).toBe(0)
                            expect(themeDeletes).toBe(0)
                            expect(
                                requests.some((request) => request.command === 'settings_update' || request.command === 'settings_set_theme'),
                            ).toBe(false)
                            await click('Close Settings')
                            await expect.poll(async () => (await previewState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            const closed = await previewState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await previewState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, baseline, previewed, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-preview-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-preview-failure.json'),
                                JSON.stringify({ state: await previewState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-preview-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsFolders) {
                        const folderSchema = catalogSchema.extend({
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            toasts: z.array(z.object({ title: z.string(), description: z.string().nullable() })),
                        })
                        const folderState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return folderSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        try {
                            await expect.poll(async () => (await folderState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            const point = (await folderState()).targets['themes-folder']
                            if (!point) throw new Error('Missing original folder button')
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                            const title =
                                "a remote session cannot open or control a window or app shown on the desktop's own display: system_open_app_data_path"
                            await expect
                                .poll(async () => (await folderState()).toasts, { timeout: DEADLINE_MS })
                                .toEqual([{ title, description: null }])
                            const denied = await folderState()
                            expect(denied.close).toBe('Open')
                            expect(active).toBe(1)
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-folders.png') })
                            await click('Unmount Settings')
                            await expect.poll(async () => (await folderState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                            await click('Close Settings')
                            await expect.poll(async () => (await folderState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(requests.filter((request) => request.command === 'system_open_app_data_path')).toHaveLength(1)
                            const closed = await folderState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await folderState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, denied, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-folders-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-folders-failure.json'),
                                JSON.stringify({ state: await folderState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-folders-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsPreferences) {
                        const preferenceSchema = catalogSchema.extend({
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            preferences: z.object({ pending: z.boolean(), followSystem: z.boolean(), results: z.array(z.string()) }).nullable(),
                            toasts: z.array(z.object({ title: z.string(), description: z.string().nullable() })),
                        })
                        const preferenceState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return preferenceSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const switchClick = async () => {
                            const point = (await preferenceState()).targets['settings.followSystemTheme']
                            if (!point) throw new Error('Missing actual Settings switch')
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        try {
                            await expect.poll(async () => (await preferenceState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            expect((await preferenceState()).preferences?.followSystem).toBe(false)
                            await control('hold-preference')
                            await switchClick()
                            await expect.poll(async () => (await preferenceState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await preferenceState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('reject-preference')
                            await expect.poll(async () => (await preferenceState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Preference')
                            await expect
                                .poll(async () => (await preferenceState()).toasts, { timeout: DEADLINE_MS })
                                .toEqual([{ title: 'settings.saveFailed', description: 'Synthetic preference refusal' }])
                            const failed = await preferenceState()
                            expect(failed.preferences?.followSystem).toBe(false)
                            expect(failed.preferences?.results).toHaveLength(1)
                            expect(failed.preferences?.results[0]).toContain('Synthetic preference refusal')
                            expect(active).toBe(1)
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-preferences.png') })
                            await click('Cancel close')
                            await expect.poll(async () => (await preferenceState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                            await control('hold-preference')
                            await switchClick()
                            await expect.poll(async () => (await preferenceState()).preferences?.pending, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await preferenceState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            await control('release-preference')
                            await expect.poll(async () => (await preferenceState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            expect(settings?.followSystemTheme).toBe(true)
                            expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(2)
                            const closed = await preferenceState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            const count = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(count)
                            expect(await preferenceState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, failed, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-preferences-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-preferences-failure.json'),
                                JSON.stringify({ state: await preferenceState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-preferences-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isSettingsTooltip) {
                        const tooltipSchema = catalogSchema.extend({
                            editing: z.string().nullable(),
                            tooltipTargets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            openTooltips: z.array(z.string()),
                        })
                        const tooltipState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return tooltipSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        try {
                            await expect.poll(async () => (await tooltipState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            const create = (await tooltipState()).targets['create-theme']
                            if (!create) throw new Error('Missing original Create theme button')
                            await page.locator('#canvas').click({ position: { x: create[0], y: create[1] } })
                            await expect.poll(async () => (await tooltipState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                            const picker = (await tooltipState()).tooltipTargets['themeEditor.pickColor']
                            if (!picker) throw new Error('Missing original color picker trigger')
                            await page.locator('#canvas').hover({ position: { x: picker[0], y: picker[1] } })
                            await expect
                                .poll(async () => (await tooltipState()).openTooltips, { timeout: DEADLINE_MS })
                                .toContain('themeEditor.pickColor')
                            const opened = await tooltipState()
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-tooltip.png') })
                            await page.keyboard.press('Escape')
                            await expect.poll(async () => (await tooltipState()).openTooltips, { timeout: DEADLINE_MS }).toEqual([])
                            await click('Unmount Settings')
                            await expect.poll(async () => (await tooltipState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                            expect((await tooltipState()).openTooltips).toEqual([])
                            await click('Close Settings')
                            await expect.poll(async () => (await tooltipState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            const closed = await tooltipState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            expect(themeSaves).toBe(0)
                            expect(themeDeletes).toBe(0)
                            const requestCount = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(requestCount)
                            expect(await tooltipState()).toEqual(closed)
                            expect(errors).toEqual([])
                            await click('Drop')
                            const result = { status: 'passed', mode, opened, closed, active, requests, errors }
                            await Bun.write(resolve(fixture, 'settings-tooltip-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'settings-tooltip-failure.json'),
                                JSON.stringify({ state: await tooltipState(), requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-tooltip-failure.png') })
                            throw error
                        }
                        return
                    }
                    if (isThemeOperations || isSettingsToast) {
                        const themeStateSchema = catalogSchema.extend({
                            editing: z.string().nullable(),
                            pendingTheme: z.boolean(),
                            themeErrors: z.array(z.string()),
                            targets: z.record(z.string(), z.tuple([z.number(), z.number()])),
                            toasts: z.array(
                                z.object({
                                    title: z.string(),
                                    description: z.string().nullable(),
                                    dismissed: z.boolean(),
                                    close: z.tuple([z.number(), z.number()]).nullable(),
                                }),
                            ),
                            reducedMotion: z.boolean(),
                        })
                        const themeState = async () => {
                            await page.locator('#state').dispatchEvent('snapshot')
                            return themeStateSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                        }
                        const canvasClick = async (key: string) => {
                            await expect.poll(async () => Object.hasOwn((await themeState()).targets, key), { timeout: DEADLINE_MS }).toBe(true)
                            const point = (await themeState()).targets[key]
                            if (!point) throw new Error(`Missing actual widget ${key}`)
                            await page.locator('#canvas').click({ position: { x: point[0], y: point[1] } })
                        }
                        if (isSettingsToast) {
                            try {
                                await expect.poll(async () => (await themeState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                                await canvasClick('create-theme')
                                await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                                await control('hold-theme')
                                await canvasClick('save')
                                await expect.poll(async () => (await themeState()).pendingTheme, { timeout: DEADLINE_MS }).toBe(true)
                                await click('Close Settings')
                                await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                                await control('reject-theme')
                                await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Theme')
                                await expect
                                    .poll(async () => (await themeState()).toasts.map((toast) => toast.title), { timeout: DEADLINE_MS })
                                    .toEqual(['Synthetic theme save refusal'])
                                const requestCount = requests.length
                                await page.emulateMedia({ reducedMotion: 'reduce' })
                                await expect.poll(async () => (await themeState()).reducedMotion, { timeout: DEADLINE_MS }).toBe(true)
                                expect(requests).toHaveLength(requestCount)
                                const failed = await themeState()
                                expect(failed.toasts[0]?.description).toBeNull()
                                expect(failed.themeErrors).toHaveLength(1)
                                expect(active).toBe(1)
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-toast.png') })
                                const disabledClose = failed.toasts[0]?.close
                                if (!disabledClose) throw new Error('Missing original Toast close hit target')
                                await page.locator('#canvas').click({ position: { x: disabledClose[0], y: disabledClose[1] } })
                                expect((await themeState()).toasts[0]?.dismissed).toBe(false)
                                await click('Cancel close')
                                await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                                const closePoint = (await themeState()).toasts[0]?.close
                                if (!closePoint) throw new Error('Missing original active Toast close hit target')
                                await page.locator('#canvas').click({ position: { x: closePoint[0], y: closePoint[1] } })
                                await expect.poll(async () => (await themeState()).toasts, { timeout: DEADLINE_MS }).toEqual([])
                                await canvasClick('back')
                                await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBeNull()
                                await page.emulateMedia({ reducedMotion: 'no-preference' })
                                await expect.poll(async () => (await themeState()).reducedMotion, { timeout: DEADLINE_MS }).toBe(false)
                                await click('Close Settings')
                                await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                                const closed = await themeState()
                                expect(closed.failures).toEqual([])
                                expect(closed.panicked).toBe(false)
                                expect(themeSaves).toBe(1)
                                expect(themeDeletes).toBe(0)
                                expect(storedTheme).toBeNull()
                                const finalCount = requests.length
                                await page.waitForTimeout(QUIET_MS)
                                expect(requests).toHaveLength(finalCount)
                                expect(await themeState()).toEqual(closed)
                                expect(errors).toEqual([])
                                await click('Drop')
                                const result = { status: 'passed', mode, failed, closed, active, requests, errors }
                                await Bun.write(resolve(fixture, 'settings-toast-result.json'), JSON.stringify(result, null, 2))
                                console.info(JSON.stringify(result))
                            } catch (error) {
                                await Bun.write(
                                    resolve(fixture, 'settings-toast-failure.json'),
                                    JSON.stringify({ state: await themeState(), requests, errors }, null, 2),
                                )
                                await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-toast-failure.png') })
                                throw error
                            }
                            return
                        }
                        try {
                            await expect.poll(async () => (await themeState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                            await canvasClick('create-theme')
                            await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                            await canvasClick('name')
                            await page.keyboard.press('Meta+A')
                            await page.keyboard.insertText('Synthetic lifecycle')
                            await control('hold-theme')
                            await canvasClick('save')
                            await expect.poll(async () => (await themeState()).pendingTheme, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            expect(active).toBe(1)
                            await control('reject-theme')
                            await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Theme')
                            const failed = await themeState()
                            expect(failed.themeErrors).toHaveLength(1)
                            expect(failed.themeErrors[0]).toContain('Synthetic theme save refusal')
                            expect(active).toBe(1)
                            await click('Cancel close')
                            await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Open')
                            await canvasClick('save')
                            await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBeNull()
                            await expect.poll(() => storedTheme?.name, { timeout: DEADLINE_MS }).toBe('Synthetic lifecycle')
                            await canvasClick('edit-custom-theme')
                            await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                            await canvasClick('name')
                            await page.keyboard.press('Meta+A')
                            await page.keyboard.insertText('Edited lifecycle')
                            await canvasClick('save')
                            await expect.poll(() => storedTheme?.name, { timeout: DEADLINE_MS }).toBe('Edited lifecycle')
                            await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBeNull()
                            await canvasClick('edit-custom-theme')
                            await expect.poll(async () => (await themeState()).editing, { timeout: DEADLINE_MS }).toBe('synthetic')
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'theme-operations.png') })
                            await canvasClick('delete')
                            await control('hold-theme')
                            await canvasClick('dialog-confirm')
                            await expect.poll(async () => (await themeState()).pendingTheme, { timeout: DEADLINE_MS }).toBe(true)
                            await click('Close Settings')
                            await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Pending')
                            expect(active).toBe(1)
                            await control('release-theme')
                            await expect.poll(async () => (await themeState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                            const closed = await themeState()
                            expect(closed.failures).toEqual([])
                            expect(closed.panicked).toBe(false)
                            expect(closed.themeErrors).toHaveLength(1)
                            expect(themeSaves).toBe(THEME_SAVES)
                            expect(themeDeletes).toBe(THEME_DELETES)
                            expect(storedTheme).toBeNull()
                            expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                            const requestCount = requests.length
                            await page.waitForTimeout(QUIET_MS)
                            expect(requests).toHaveLength(requestCount)
                            expect(await themeState()).toEqual(closed)
                            await click('Drop')
                            expect(errors).toEqual([])
                            const result = { status: 'passed', mode, failed, closed, themeSaves, themeDeletes, upgrades, active, requests, errors }
                            await Bun.write(resolve(fixture, 'theme-operations-result.json'), JSON.stringify(result, null, 2))
                            console.info(JSON.stringify(result))
                        } catch (error) {
                            await Bun.write(
                                resolve(fixture, 'theme-operations-failure.json'),
                                JSON.stringify({ state: await themeState(), storedTheme, requests, errors }, null, 2),
                            )
                            await page.locator('#canvas').screenshot({ path: resolve(fixture, 'theme-operations-failure.png') })
                            throw error
                        }
                        return
                    }
                    try {
                        await expect.poll(async () => (await catalogState()).catalogErrors.length, { timeout: DEADLINE_MS }).toBe(1)
                    } catch (error) {
                        await Bun.write(
                            resolve(fixture, 'settings-catalog-failure.json'),
                            JSON.stringify({ state: await catalogState(), requests, errors }, null, 2),
                        )
                        throw error
                    }
                    const boot = await catalogState()
                    expect(boot.themes).toEqual(['taide-dark'])
                    expect(boot.catalogErrors[0]).toContain('Synthetic locale refusal')
                    expect(boot.failures).toEqual([])
                    expect(themeLists).toBe(1)
                    expect(localeLists).toBe(1)
                    if (isSettingsVisual) {
                        await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-visual.png') })
                        await click('Drop')
                        await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                        expect(boot.panicked).toBe(false)
                        expect(errors).toEqual([])
                        const result = { status: 'passed', mode, boot, active, requests, errors }
                        await Bun.write(resolve(fixture, 'settings-visual-result.json'), JSON.stringify(result, null, 2))
                        console.info(JSON.stringify(result))
                        return
                    }
                    await control('hold-catalog')
                    await control('theme-event')
                    await expect.poll(() => themeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_HELD_READS)
                    await expect.poll(() => localeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_HELD_READS)
                    await click('Unmount Settings')
                    await expect.poll(async () => (await catalogState()).mount, { timeout: DEADLINE_MS }).toBeNull()
                    await click('Mount Settings')
                    await expect.poll(() => themeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_REMOUNT_READS)
                    await expect.poll(() => localeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_REMOUNT_READS)
                    await control('release-catalog')
                    await expect.poll(async () => (await catalogState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                    const remounted = await catalogState()
                    expect(remounted.mount).toBeGreaterThan(boot.mount ?? 0)
                    expect(remounted.themes).toEqual(['taide-dark'])
                    expect(remounted.catalogErrors).toEqual([])
                    await control('theme-event')
                    await expect.poll(() => themeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_FINAL_READS)
                    await expect.poll(() => localeLists, { timeout: DEADLINE_MS }).toBe(CATALOG_FINAL_READS)
                    await expect.poll(async () => (await catalogState()).locales, { timeout: DEADLINE_MS }).toEqual(['en'])
                    const refreshed = await catalogState()
                    expect(refreshed.mount).toBe(remounted.mount)
                    expect(refreshed.generation).toBeGreaterThan(remounted.generation ?? 0)
                    await page.locator('#canvas').screenshot({ path: resolve(fixture, 'settings-catalog.png') })
                    await click('Close Settings')
                    await expect.poll(async () => (await catalogState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    const closed = await catalogState()
                    const requestCount = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(requestCount)
                    expect(await catalogState()).toEqual(closed)
                    expect(closed.failures).toEqual([])
                    expect(closed.panicked).toBe(false)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = {
                        status: 'passed',
                        mode,
                        boot,
                        remounted,
                        refreshed,
                        closed,
                        themeLists,
                        localeLists,
                        upgrades,
                        active,
                        requests,
                        errors,
                    }
                    await Bun.write(resolve(fixture, 'settings-catalog-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isCanvas) {
                    await expect(page.locator('#state')).toHaveAttribute('data-ready', 'true', { timeout: DEADLINE_MS })
                    const canvasSchema = z.object({
                        frames: z.number().int().nonnegative(),
                        pumps: z.number().int().nonnegative(),
                        textInputs: z.number().int().nonnegative(),
                        text: z.string().nullable(),
                        dirty: z.boolean(),
                        failures: z.array(z.string()),
                        panicked: z.boolean(),
                        close: z.string(),
                    })
                    const canvasState = async () => {
                        await page.locator('#state').dispatchEvent('snapshot')
                        return canvasSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                    }
                    await expect.poll(async () => (await canvasState()).text, { timeout: DEADLINE_MS }).toBe('disk')
                    await expect.poll(async () => (await canvasState()).frames, { timeout: DEADLINE_MS }).toBeGreaterThan(0)
                    const initial = await canvasState()
                    expect(initial.close).toBe('Open')
                    if (isCanvasAbort) {
                        await click('Abort canvas')
                        await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                        const aborted = await canvasState()
                        expect(aborted.close).toBe('Disposed')
                        const finalRequests = requests.length
                        await page.locator('body').dispatchEvent('blur')
                        await page.waitForTimeout(QUIET_MS)
                        expect(await canvasState()).toEqual(aborted)
                        expect(requests).toHaveLength(finalRequests)
                        expect(aborted.failures).toEqual([])
                        expect(aborted.panicked).toBe(false)
                        await click('Drop')
                        expect(errors).toEqual([])
                        const result = { status: 'passed', initial, aborted, upgrades, active, requests, errors }
                        await Bun.write(resolve(fixture, 'canvas-abort-result.json'), JSON.stringify(result, null, 2))
                        console.info(JSON.stringify(result))
                        return
                    }
                    if (isCanvasVisual) {
                        await page.locator('#canvas').screenshot({ path: resolve(fixture, 'canvas-visual.png') })
                        await click('Drop')
                        await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                        const finalRequests = requests.length
                        await page.waitForTimeout(QUIET_MS)
                        expect(requests).toHaveLength(finalRequests)
                        expect(initial.failures).toEqual([])
                        expect(initial.panicked).toBe(false)
                        expect(errors).toEqual([])
                        const result = { status: 'passed', initial, upgrades, active, requests, errors }
                        await Bun.write(resolve(fixture, 'canvas-visual-result.json'), JSON.stringify(result, null, 2))
                        console.info(JSON.stringify(result))
                        return
                    }
                    await page.locator('#canvas').click()
                    await page.keyboard.press('Home')
                    await page.keyboard.insertText(TYPED_TEXT)
                    await expect.poll(async () => (await canvasState()).text, { timeout: DEADLINE_MS }).toBe('typed disk')
                    await expect.poll(() => currentMirror?.entry.content, { timeout: DEADLINE_MS }).toBe('typed disk')
                    const edited = await canvasState()
                    expect(edited.textInputs).toBeGreaterThan(initial.textInputs)
                    expect(edited.dirty).toBe(true)
                    expect(mirrorWrites).toBe(1)
                    await page.locator('#canvas').screenshot({ path: resolve(fixture, 'canvas.png') })
                    await click('Close canvas')
                    await expect.poll(async () => (await canvasState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    const closed = await canvasState()
                    const finalRequests = requests.length
                    await page.locator('body').dispatchEvent('blur')
                    await page.waitForTimeout(QUIET_MS)
                    expect(await canvasState()).toEqual(closed)
                    expect(requests).toHaveLength(finalRequests)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    expect(closed.failures).toEqual([])
                    expect(closed.panicked).toBe(false)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', initial, edited, closed, mirrorWrites, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'canvas-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isApplication) {
                    await expect(page.locator('#state')).toHaveAttribute('data-ready', 'true')
                    const applicationSchema = z.object({
                        disposed: z.boolean(),
                        close: z.string(),
                        pendingOperations: z.boolean().optional(),
                        dirtyReady: z.boolean().optional(),
                        ready: z.boolean().optional(),
                        text: z.string().nullable().optional(),
                        dirty: z.boolean().nullable().optional(),
                        connected: z.boolean().optional(),
                        scopeReady: z.boolean().optional(),
                        mirrorReady: z.boolean().optional(),
                        mirrorStatus: z.string().optional(),
                        timerArmed: z.boolean().optional(),
                        schedulerFailed: z.boolean().optional(),
                        recoveries: z.number().optional(),
                        loaded: z.number().optional(),
                        pumps: z.number().int().nonnegative(),
                        changes: z.number().int().nonnegative(),
                        failures: z.array(z.string()),
                        responses: z.array(z.string()),
                        preferences: z.array(z.string()),
                        settingsEvents: z.number().int().nonnegative(),
                        pendingPreferences: z.boolean().optional(),
                        showSystemUsage: z.boolean().nullable().optional(),
                        language: z.string().nullable().optional(),
                        localeId: z.string().nullable().optional(),
                    })
                    const applicationState = async () => {
                        await page.locator('#state').dispatchEvent('snapshot')
                        return applicationSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                    }
                    await expect.poll(async () => (await applicationState()).ready, { timeout: DEADLINE_MS }).toBe(true)
                    await expect.poll(async () => (await applicationState()).scopeReady, { timeout: DEADLINE_MS }).toBe(true)
                    const initial = await applicationState()
                    expect(initial.text).toBe('disk')
                    if (isPreferences) {
                        expect(initial.preferences).toEqual([])
                        await click('Toggle setting')
                        await expect.poll(async () => (await applicationState()).preferences.length, { timeout: DEADLINE_MS }).toBe(1)
                        const switched = await applicationState()
                        expect(switched.showSystemUsage).toBe(!initial.showSystemUsage)
                        expect(switched.preferences[0]).toContain(':ok')
                        await control('hold-preference')
                        await click('Change language')
                        await expect.poll(async () => (await applicationState()).pendingPreferences, { timeout: DEADLINE_MS }).toBe(true)
                        await control('external-preference')
                        await expect.poll(async () => (await applicationState()).localeId, { timeout: DEADLINE_MS }).toBe('en')
                        await control('release-preference')
                        await expect
                            .poll(async () => (await applicationState()).preferences.length, { timeout: DEADLINE_MS })
                            .toBe(PREFERENCE_STAGE.LANGUAGE)
                        const external = await applicationState()
                        expect(external.language).toBe('en')
                        expect(external.localeId).toBe('en')
                        await control('hold-preference')
                        await click('Fail theme')
                        await expect.poll(async () => (await applicationState()).pendingPreferences, { timeout: DEADLINE_MS }).toBe(true)
                        await click('Request close')
                        expect((await applicationState()).close).toBe('Pending')
                        await control('release-preference')
                        await expect.poll(async () => (await applicationState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Preference(')
                        const failed = await applicationState()
                        expect(failed.preferences).toHaveLength(PREFERENCE_STAGE.FAILED)
                        expect(failed.preferences.at(-1)).toContain('SYNTHETIC_SETTINGS')
                        expect(failed.connected).toBe(true)
                        expect(active).toBe(1)
                        await click('Cancel close')
                        await click('Change theme')
                        await expect
                            .poll(async () => (await applicationState()).preferences.length, { timeout: DEADLINE_MS })
                            .toBe(PREFERENCE_STAGE.THEME)
                        await control('hold-preference')
                        await click('Toggle setting')
                        await expect.poll(async () => (await applicationState()).pendingPreferences, { timeout: DEADLINE_MS }).toBe(true)
                        await click('Cut connection')
                        await expect
                            .poll(async () => (await applicationState()).preferences.length, { timeout: DEADLINE_MS })
                            .toBe(PREFERENCE_STAGE.CLOSED)
                        expect((await applicationState()).preferences.at(-1)).toContain('Closed')
                        const updateCount = requests.filter((request) => request.command === 'settings_update').length
                        await expect.poll(async () => (await applicationState()).recoveries, { timeout: DEADLINE_MS }).toBe(1)
                        await expect.poll(async () => (await applicationState()).connected, { timeout: DEADLINE_MS }).toBe(true)
                        const recovered = await applicationState()
                        expect(recovered.pendingPreferences).toBe(false)
                        expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(updateCount)
                        await click('Toggle setting')
                        await expect
                            .poll(async () => (await applicationState()).preferences.length, { timeout: DEADLINE_MS })
                            .toBe(PREFERENCE_STAGE.RETRIED)
                        expect((await applicationState()).showSystemUsage).toBe(!recovered.showSystemUsage)
                        await control('hold-preference')
                        await click('Change theme')
                        await expect.poll(async () => (await applicationState()).pendingPreferences, { timeout: DEADLINE_MS }).toBe(true)
                        await click('Request close')
                        const pending = await applicationState()
                        expect(pending.close).toBe('Pending')
                        expect(pending.connected).toBe(true)
                        expect(active).toBe(1)
                        await control('release-preference')
                        await expect.poll(async () => (await applicationState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                        await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                        const closed = await applicationState()
                        expect(closed.preferences).toHaveLength(PREFERENCE_STAGE.DRAINED)
                        expect(closed.preferences.at(-1)).toContain(':ok')
                        const finalRequests = requests.length
                        await page.waitForTimeout(QUIET_MS)
                        expect(await applicationState()).toEqual(closed)
                        expect(requests).toHaveLength(finalRequests)
                        expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                        expect(requests.filter((request) => request.command === 'settings_update')).toHaveLength(PREFERENCE_UPDATES)
                        expect(requests.filter((request) => request.command === 'settings_set_theme')).toHaveLength(PREFERENCE_THEMES)
                        expect(closed.responses).toEqual([])
                        expect(closed.settingsEvents).toBe(1)
                        expect(closed.failures).toEqual([])
                        await click('Drop')
                        expect(errors).toEqual([])
                        const result = {
                            status: 'passed',
                            initial,
                            switched,
                            external,
                            failed,
                            recovered,
                            pending,
                            closed,
                            upgrades,
                            active,
                            requests,
                            errors,
                        }
                        await Bun.write(resolve(fixture, 'preferences-result.json'), JSON.stringify(result, null, 2))
                        console.info(JSON.stringify(result))
                        return
                    }
                    await click('Bind scope')
                    await expect.poll(async () => (await applicationState()).mirrorReady, { timeout: DEADLINE_MS }).toBe(true)
                    if (isApplicationClose) {
                        await control('hold-write')
                        await control('fail-dirty')
                        await click('Edit')
                        await click('Request close')
                        await expect.poll(async () => (await applicationState()).close, { timeout: DEADLINE_MS }).toContain('Failed(Dirty(')
                        const failed = await applicationState()
                        expect(failed.connected).toBe(true)
                        expect(active).toBe(1)
                        await click('Try update while closing')
                        await expect(page.locator('#closing-result')).toHaveText('true')
                        await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(1)
                        await click('Cancel close')
                        expect((await applicationState()).close).toBe('Open')
                        await control('release-write')
                        await click('Retry persistence')
                        await expect.poll(async () => (await applicationState()).dirtyReady, { timeout: DEADLINE_MS }).toBe(true)
                        await expect.poll(async () => (await applicationState()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                        await control('hold-read')
                        await click('Keep draft')
                        await expect.poll(async () => (await applicationState()).pendingOperations, { timeout: DEADLINE_MS }).toBe(true)
                        await click('Request close')
                        expect((await applicationState()).close).toBe('Pending')
                        expect(active).toBe(1)
                        await click('Cancel close')
                        await control('release-read')
                        await expect.poll(async () => (await applicationState()).pendingOperations, { timeout: DEADLINE_MS }).toBe(false)
                        await control('hold-save')
                        await control('hold-write')
                        await click('Edit')
                        await click('Save')
                        await expect
                            .poll(() => requests.filter((request) => request.command === 'file_save').length, { timeout: DEADLINE_MS })
                            .toBe(1)
                        await click('Request close')
                        await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(SHARED_VIEWS)
                        const pending = await applicationState()
                        expect(pending.close).toBe('Pending')
                        expect(pending.pendingOperations).toBe(true)
                        expect(pending.mirrorStatus).toBe('Pending')
                        await control('release-write')
                        await expect.poll(async () => (await applicationState()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                        const awaitingSave = await applicationState()
                        expect(awaitingSave.close).toBe('Pending')
                        expect(awaitingSave.pendingOperations).toBe(true)
                        expect(active).toBe(1)
                        await control('hold-dirty')
                        await control('release-save')
                        await expect.poll(async () => (await applicationState()).pendingOperations, { timeout: DEADLINE_MS }).toBe(false)
                        const awaitingDirty = await applicationState()
                        expect(awaitingDirty.close).toBe('Pending')
                        expect(awaitingDirty.dirtyReady).toBe(false)
                        expect(awaitingDirty.dirty).toBe(false)
                        expect(active).toBe(1)
                        await control('release-dirty')
                        await expect.poll(async () => (await applicationState()).close, { timeout: DEADLINE_MS }).toBe('Ready')
                        await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                        const closed = await applicationState()
                        expect(closed.disposed).toBe(true)
                        const requestCount = requests.length
                        await page.evaluate(() => window.dispatchEvent(new Event('blur')))
                        await page.waitForTimeout(QUIET_MS)
                        expect(await applicationState()).toEqual(closed)
                        expect(requests).toHaveLength(requestCount)
                        expect(requests.map((request) => request.seq)).toEqual(Array.from({ length: requestCount }, (_, index) => index + 1))
                        expect(upgrades).toBe(1)
                        expect(errors).toEqual([])
                        expect(closed.failures).toEqual([])
                        expect(closed.responses).toEqual([])
                        expect(disk).toBe(`${TYPED_TEXT}${TYPED_TEXT}disk`)
                        expect(currentMirror).toBeNull()
                        await click('Drop')
                        await Bun.write(
                            resolve(fixture, 'application-close-result.json'),
                            JSON.stringify(
                                {
                                    status: 'passed',
                                    mode,
                                    initial,
                                    failed,
                                    pending,
                                    awaitingSave,
                                    awaitingDirty,
                                    closed,
                                    mirrorWrites,
                                    mirrorReads,
                                    upgrades,
                                    active,
                                    requests,
                                    errors,
                                },
                                null,
                                2,
                            ),
                        )
                        console.log({
                            status: 'passed',
                            mode,
                            mirrorWrites,
                            mirrorReads,
                            upgrades,
                            active,
                            requests: requestCount,
                            pumps: closed.pumps,
                            changes: closed.changes,
                        })
                        return
                    }
                    await click('Nested update')
                    await expect(page.locator('#busy-result')).toHaveText('true')
                    await click('Edit')
                    await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(1)
                    await expect.poll(async () => (await applicationState()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    const written = await applicationState()
                    expect(written.text).toBe(`${TYPED_TEXT}disk`)
                    expect(written.dirty).toBe(true)
                    expect(currentMirror?.entry.content).toBe(`${TYPED_TEXT}disk`)
                    await click('Edit')
                    await page.evaluate(() => window.dispatchEvent(new Event('blur')))
                    await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(SHARED_VIEWS)
                    await expect.poll(async () => (await applicationState()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    const blurred = await applicationState()
                    expect(blurred.text).toBe(`${TYPED_TEXT}${TYPED_TEXT}disk`)
                    expect(blurred.timerArmed).toBe(false)
                    expect(blurred.schedulerFailed).toBe(false)
                    expect(blurred.responses).toEqual([])
                    expect(blurred.failures).toEqual([])
                    await page.waitForTimeout(QUIET_MS)
                    expect(await applicationState()).toEqual(blurred)
                    await click('Edit')
                    await click('Dispose')
                    const disposed = await applicationState()
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    const requestCount = requests.length
                    await page.evaluate(() => window.dispatchEvent(new Event('blur')))
                    await page.waitForTimeout(QUIET_MS)
                    expect(await applicationState()).toEqual(disposed)
                    expect(mirrorWrites).toBe(SHARED_VIEWS)
                    expect(requests).toHaveLength(requestCount)
                    expect(upgrades).toBe(1)
                    expect(disposed.disposed).toBe(true)
                    expect(errors).toEqual([])
                    await click('Drop')
                    await Bun.write(
                        resolve(fixture, 'application-pump-result.json'),
                        JSON.stringify(
                            {
                                status: 'passed',
                                mode,
                                initial,
                                written,
                                blurred,
                                disposed,
                                mirrorWrites,
                                mirrorReads,
                                upgrades,
                                active,
                                requests,
                                errors,
                            },
                            null,
                            2,
                        ),
                    )
                    console.log({
                        status: 'passed',
                        mode,
                        mirrorWrites,
                        mirrorReads,
                        upgrades,
                        active,
                        requests: requests.length,
                        pumps: disposed.pumps,
                        changes: disposed.changes,
                    })
                    return
                }
                await expect
                    .poll(
                        async () => {
                            const parsed = stateSchema.safeParse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                            return parsed.success && parsed.data.rendered.includes(isPreparedConfig ? 'disk  ' : 'disk')
                        },
                        { timeout: DEADLINE_MS },
                    )
                    .toBe(true)
                const boot = await state()
                expect(boot.documents).toBe(1)
                expect(boot.views).toBe(SHARED_VIEWS)
                expect(boot.dirty).toBe(false)
                expect(boot.readOnly).toBe(false)
                if (isMirrorWrite) {
                    await expect.poll(async () => (await state()).scopeReady, { timeout: DEADLINE_MS }).toBe(true)
                    await click('Bind project scopes')
                    await expect.poll(async () => (await state()).mirrorReady, { timeout: DEADLINE_MS }).toBe(true)
                    expect((await state()).mirrorTexts).toEqual([])
                    await control('hold-write')
                    await click('Actual text input')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe(`${TYPED_TEXT}disk`)
                    await expect.poll(async () => (await state()).dirtyFlushed, { timeout: DEADLINE_MS }).toBe(true)
                    await expect.poll(async () => (await state()).mirrorTimerArmed, { timeout: DEADLINE_MS }).toBe(true)
                    await click('Pause polls')
                    const diagnostic = async () => {
                        await click('Read mirror diagnostic')
                        return z
                            .object({ wakes: z.number(), timerArmed: z.boolean(), status: z.string() })
                            .parse(JSON.parse((await page.locator('#mirror-diagnostic').textContent()) ?? '{}'))
                    }
                    const beforeTimer = await diagnostic()
                    expect(beforeTimer.timerArmed).toBe(true)
                    await page.waitForTimeout(QUIET_MS)
                    const afterTimer = await diagnostic()
                    expect(afterTimer.wakes).toBe(beforeTimer.wakes + 1)
                    expect(afterTimer.timerArmed).toBe(false)
                    expect(mirrorWrites).toBe(0)
                    await click('Resume polls')
                    await expect.poll(() => deferredWrite.length, { timeout: DEADLINE_MS }).toBe(1)
                    await click('Save prepared')
                    await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([false])
                    expect((await state()).dirty).toBe(false)
                    await control('release-write')
                    await expect.poll(() => mirrorClears, { timeout: DEADLINE_MS }).toBe(1)
                    await expect.poll(async () => (await state()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    await expect.poll(async () => (await state()).mirrorTexts, { timeout: DEADLINE_MS }).toEqual([])
                    expect(currentMirror).toBeNull()
                    const cleaned = await state()
                    await click('Actual text input')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe(`${TYPED_TEXT}${TYPED_TEXT}disk`)
                    await click('Flush mirrors')
                    await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(SHARED_VIEWS)
                    await expect.poll(async () => (await state()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    expect((await state()).mirrorTexts).toEqual([`${TYPED_TEXT}${TYPED_TEXT}disk`])
                    await click('Flush mirrors')
                    expect(mirrorWrites).toBe(SHARED_VIEWS)
                    await click('Actual text input')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe(`${TYPED_TEXT}${TYPED_TEXT}${TYPED_TEXT}disk`)
                    await click('Unbind first')
                    await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(UNMOUNT_MIRROR_WRITES)
                    await click('Unbind second')
                    await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    const unmounted = await state()
                    expect(unmounted.documents).toBe(1)
                    await click('Bind project scopes')
                    await expect.poll(async () => (await state()).mirrorReady, { timeout: DEADLINE_MS }).toBe(true)
                    expect((await state()).restored).toBe(0)
                    await click('Pending edit then reconnect')
                    await expect.poll(async () => (await state()).recoveries, { timeout: DEADLINE_MS }).toBe(1)
                    await expect.poll(async () => (await state()).mirrorFailures.length, { timeout: DEADLINE_MS }).toBe(1)
                    const recovered = await state()
                    expect(recovered.mirrorFailures[0]).toContain('Closed')
                    expect(recovered.mirrorTimerArmed).toBe(false)
                    expect(recovered.text).toBe(`${TYPED_TEXT}${TYPED_TEXT}${TYPED_TEXT}${TYPED_TEXT}disk`)
                    expect(mirrorWrites).toBe(UNMOUNT_MIRROR_WRITES)
                    await click('Retry mirror writes')
                    await expect.poll(() => mirrorWrites, { timeout: DEADLINE_MS }).toBe(FINAL_MIRROR_WRITES)
                    await expect.poll(async () => (await state()).mirrorStatus, { timeout: DEADLINE_MS }).toBe('Ready')
                    await expect.poll(async () => (await state()).mirrorTexts, { timeout: DEADLINE_MS }).toEqual([recovered.text])
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    const finalRequests = requests.length
                    await page.evaluate(() => window.dispatchEvent(new Event('blur')))
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect((await state()).mirrorFailures).toHaveLength(1)
                    expect((await state()).responses).toEqual([])
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    expect(upgrades).toBe(FINAL_UPGRADES)
                    expect(mirrorClears).toBe(1)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = {
                        status: 'passed',
                        mode,
                        boot,
                        beforeTimer,
                        afterTimer,
                        cleaned,
                        unmounted,
                        recovered,
                        disposed,
                        mirrorWrites,
                        mirrorClears,
                        mirrorReads,
                        upgrades,
                        active,
                        requests,
                        errors,
                    }
                    await Bun.write(resolve(fixture, 'mirror-write-file-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isMirrorRestore) {
                    await expect.poll(async () => (await state()).scopeReady, { timeout: DEADLINE_MS }).toBe(true)
                    await click('Bind project scopes')
                    await expect.poll(() => deferredMirror.length, { timeout: DEADLINE_MS }).toBe(1)
                    const waiting = await state()
                    expect(waiting.text).toBe('disk')
                    expect(waiting.dirty).toBe(false)
                    expect(waiting.restored).toBe(0)
                    await control('release-mirror')
                    await expect.poll(async () => (await state()).restored, { timeout: DEADLINE_MS }).toBe(1)
                    await expect.poll(async () => (await state()).dirtyFlushed, { timeout: DEADLINE_MS }).toBe(true)
                    const restored = await state()
                    expect(restored.text).toBe(RESTORED_TEXT)
                    expect(restored.dirty).toBe(true)
                    expect(restored.rendered).toContain('restored conflict')
                    await click('Actual text input')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe(`${TYPED_TEXT}${RESTORED_TEXT}`)
                    await control('hold-mirror')
                    await click('Retry mirrors')
                    await expect.poll(() => deferredMirror.length, { timeout: DEADLINE_MS }).toBe(1)
                    await click('Unbind first')
                    await click('Unbind second')
                    await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(0)
                    await click('Bind project scopes')
                    await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(SHARED_VIEWS)
                    await control('release-mirror')
                    await expect.poll(async () => (await state()).mirrorTexts, { timeout: DEADLINE_MS }).toEqual([LATEST_MIRROR_TEXT])
                    const rebound = await state()
                    expect(rebound.text).toBe(`${TYPED_TEXT}${RESTORED_TEXT}`)
                    expect(rebound.dirty).toBe(true)
                    expect(rebound.restored).toBe(1)
                    expect(rebound.documents).toBe(1)
                    expect(mirrorReads).toBe(MIRROR_READS)
                    expect(requests.filter((request) => request.command === 'layout_set_dirty')).toHaveLength(1)
                    expect(requests.filter((request) => request.command === 'file_save')).toEqual([])
                    expect(rebound.mirrorRestoreFailures).toEqual([])
                    expect(rebound.responses).toEqual([])
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, waiting, restored, rebound, disposed, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'mirror-file-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isDirtyClose) {
                    await click('Pending edit then reconnect')
                    await expect
                        .poll(
                            async () => {
                                const current = await state()
                                return (
                                    current.connected &&
                                    current.recoveries === 1 &&
                                    current.dirtyFailures.length === 1 &&
                                    current.loaded === SAVED_LOADS
                                )
                            },
                            { timeout: DEADLINE_MS },
                        )
                        .toBe(true)
                    const recovered = await state()
                    expect(recovered.text).toBe(`${TYPED_TEXT}disk`)
                    expect(recovered.dirty).toBe(true)
                    expect(recovered.dirtyFailures[0]).toContain('Closed')
                    expect(requests.filter((request) => request.command === 'layout_set_dirty')).toEqual([])
                    expect(requests.filter((request) => request.command === 'file_save')).toEqual([])
                    const recoveryRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(recoveryRequests)
                    await click('Retry dirty')
                    await expect
                        .poll(() => requests.filter((request) => request.command === 'layout_set_dirty').length, { timeout: DEADLINE_MS })
                        .toBe(1)
                    await expect.poll(async () => (await state()).dirtyFlushed, { timeout: DEADLINE_MS }).toBe(true)
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    expect(disposed.dirty).toBe(true)
                    expect(disposed.dirtyFailures).toHaveLength(1)
                    expect(disposed.responses).toEqual([])
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect(upgrades).toBe(FINAL_UPGRADES)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, recovered, disposed, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'dirty-close-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isDirtyOwner) {
                    await control('hold-dirty')
                    await click('Actual text input')
                    await expect.poll(() => deferredDirty.length, { timeout: DEADLINE_MS }).toBe(1)
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe(`${TYPED_TEXT}disk`)
                    await control('external-disk')
                    await click('Read file')
                    await expect.poll(async () => (await state()).conflict, { timeout: DEADLINE_MS }).toBe(true)
                    await click('Owned view disk')
                    await click('Actual text input')
                    const locked = await state()
                    expect(locked.text).toBe(`${TYPED_TEXT}disk`)
                    expect(requests.filter((request) => request.command === 'file_open')).toHaveLength(SHARED_VIEWS)
                    expect(deferredDirty).toHaveLength(1)
                    await control('release-dirty')
                    await expect.poll(async () => (await state()).choices, { timeout: DEADLINE_MS }).toEqual([false])
                    await expect
                        .poll(() => requests.filter((request) => request.command === 'layout_set_dirty').map((request) => request.args?.dirty), {
                            timeout: DEADLINE_MS,
                        })
                        .toEqual([true, false])
                    const chosen = await state()
                    expect(chosen.text).toBe(disk)
                    expect(chosen.dirty).toBe(false)
                    expect(chosen.conflict).toBe(false)
                    await control('fail-dirty')
                    await click('Actual text input')
                    await expect.poll(async () => (await state()).dirtyFailures.length, { timeout: DEADLINE_MS }).toBe(1)
                    const draft = (await state()).text
                    expect(draft?.length).toBe((chosen.text?.length ?? 0) + TYPED_TEXT.length)
                    await control('external-disk')
                    await click('Read file')
                    await expect.poll(async () => (await state()).conflict, { timeout: DEADLINE_MS }).toBe(true)
                    const readsBeforeFailure = requests.filter((request) => request.command === 'file_open').length
                    await click('Owned view disk')
                    await expect.poll(async () => (await state()).choiceFailures.length, { timeout: DEADLINE_MS }).toBe(1)
                    expect(requests.filter((request) => request.command === 'file_open')).toHaveLength(readsBeforeFailure)
                    expect((await state()).text).toBe(draft)
                    expect((await state()).dirty).toBe(true)
                    await click('Retry dirty')
                    await click('Owned keep draft')
                    await expect.poll(async () => (await state()).choices, { timeout: DEADLINE_MS }).toEqual([false, true])
                    const kept = await state()
                    expect(kept.text).toBe(draft)
                    expect(kept.dirty).toBe(true)
                    expect(kept.conflict).toBe(false)
                    await click('Save prepared')
                    await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([false])
                    await expect
                        .poll(() => requests.filter((request) => request.command === 'file_open').length, { timeout: DEADLINE_MS })
                        .toBe(OWNER_READS)
                    await expect
                        .poll(() => requests.filter((request) => request.command === 'layout_set_dirty').length, { timeout: DEADLINE_MS })
                        .toBe(OWNER_DIRTY_REQUESTS)
                    const saved = await state()
                    expect(saved.dirty).toBe(false)
                    expect(saved.text).toBe(disk)
                    expect(saved.dirtyFlushes).toBe(0)
                    expect(saved.responses).toEqual([])
                    expect(saved.dirtyFailures).toHaveLength(1)
                    expect(saved.choiceFailures).toHaveLength(1)
                    expect(requests.filter((request) => request.command === 'layout_set_dirty').map((request) => request.args?.dirty)).toEqual([
                        true,
                        false,
                        true,
                        true,
                        false,
                    ])
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, locked, chosen, kept, saved, disposed, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'dirty-file-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isDiskChoice) {
                    await click('First edit')
                    await control('external-disk')
                    await click('Read file')
                    await expect.poll(async () => (await state()).rendered, { timeout: DEADLINE_MS }).toContain('changed on disk')
                    const conflict = await state()
                    expect(conflict.text).toBe('first disk')
                    expect(conflict.conflict).toBe(true)
                    await click('Keep draft after dirty flush')
                    await expect.poll(async () => (await state()).choices, { timeout: DEADLINE_MS }).toEqual([true])
                    const kept = await state()
                    expect(kept.text).toBe('first disk')
                    expect(kept.dirty).toBe(true)
                    expect(kept.conflict).toBe(false)
                    expect(kept.rendered).not.toContain('changed on disk')
                    await control('external-disk')
                    const freshDisk = disk
                    await click('Read file')
                    await expect.poll(async () => (await state()).conflict, { timeout: DEADLINE_MS }).toBe(true)
                    await click('View disk after dirty flush')
                    await expect.poll(async () => (await state()).choices, { timeout: DEADLINE_MS }).toEqual([true, false])
                    const chosen = await state()
                    expect(chosen.text).toBe(freshDisk)
                    expect(chosen.dirty).toBe(false)
                    expect(chosen.conflict).toBe(false)
                    expect(chosen.rendered).toContain(freshDisk)
                    expect(chosen.rendered).not.toContain('changed on disk')
                    expect(chosen.dirtyFlushes).toBe(DIRTY_FLUSHES)
                    expect(chosen.choiceFailures).toEqual([])
                    expect(chosen.responses).toEqual([])
                    expect(requests.filter((request) => request.command === 'file_open')).toHaveLength(CHOICE_READS)
                    expect(requests.filter((request) => request.command === 'file_save')).toEqual([])
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, conflict, kept, chosen, disposed, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'choice-file-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                if (isPreparedConfig) {
                    await click('Save prepared')
                    expect(requests.filter((request) => request.command === 'file_save')).toEqual([])
                    await click('Insert tab')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe('  disk  \r\n')
                    await click('Save prepared')
                    await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([false])
                    await expect.poll(async () => (await state()).loaded, { timeout: DEADLINE_MS }).toBe(SAVED_LOADS)
                    const overridden = await state()
                    expect(disk).toBe('  disk  \r\n')
                    expect(overridden.dirty).toBe(false)
                    await control('enable-cleanup')
                    await click('Read file')
                    await expect.poll(async () => (await state()).loaded, { timeout: DEADLINE_MS }).toBe(RECOVERED_LOADS)
                    await click('Insert tab')
                    await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe('    disk  \r\n')
                    await click('Save prepared')
                    await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([false, false])
                    await expect.poll(async () => (await state()).loaded, { timeout: DEADLINE_MS }).toBe(FINAL_LOADS)
                    const cleaned = await state()
                    expect(cleaned.text).toBe('    disk\r\n')
                    expect(cleaned.dirty).toBe(false)
                    expect(cleaned.conflict).toBe(false)
                    expect(disk).toBe('    disk\r\n')
                    expect(requests.filter((request) => request.command === 'file_save').map((request) => request.args)).toEqual([
                        { path: PATH, content: '  disk  \r\n' },
                        { path: PATH, content: '    disk\r\n' },
                    ])
                    expect(requests.filter((request) => request.command === 'file_open')).toHaveLength(PREPARED_READS)
                    expect(requests.filter((request) => request.command === 'file_save')).toHaveLength(PREPARED_SAVES)
                    await click('Dispose')
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    await expect.poll(async () => (await state()).connected, { timeout: DEADLINE_MS }).toBe(false)
                    const disposed = await state()
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect((await state()).wakes).toBe(disposed.wakes)
                    expect(upgrades).toBe(1)
                    expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                    expect((await state()).responses).toEqual([])
                    await click('Drop')
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, overridden, cleaned, disposed, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'prepared-file-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }
                await click('First edit')
                await control('hold-read')
                await click('Read file')
                await expect.poll(() => deferredRead.length, { timeout: DEADLINE_MS }).toBe(1)
                await control('hold-save')
                await click('Save prepared')
                await expect.poll(() => deferredSave.length, { timeout: DEADLINE_MS }).toBe(1)
                await click('Save prepared')
                expect(deferredSave).toHaveLength(1)
                await click('Later edit')
                await expect.poll(async () => (await state()).text, { timeout: DEADLINE_MS }).toBe('first later disk')
                await control('release-save')
                await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([true])
                await control('release-read')
                await expect.poll(async () => (await state()).loaded, { timeout: DEADLINE_MS }).toBe(SAVED_LOADS)
                const saved = await state()
                expect(disk).toBe('first disk')
                expect(saved.text).toBe('first later disk')
                expect(saved.dirty).toBe(true)
                expect(saved.conflict).toBe(false)
                expect(saved.rendered).toContain('first later disk')
                await control('hold-save')
                await click('Save prepared')
                await expect.poll(() => deferredSave.length, { timeout: DEADLINE_MS }).toBe(1)
                await click('Reconnect')
                await expect
                    .poll(
                        async () => {
                            const current = await state()
                            return (
                                current.connected &&
                                current.recoveries === 1 &&
                                current.loaded === RECOVERED_LOADS &&
                                current.saveFailures.length === 1
                            )
                        },
                        { timeout: DEADLINE_MS },
                    )
                    .toBe(true)
                const recovered = await state()
                expect(recovered.saveFailures[0]).toContain('Closed')
                expect(recovered.saves).toEqual([true])
                expect(recovered.text).toBe('first later disk')
                expect(recovered.dirty).toBe(true)
                expect(recovered.conflict).toBe(false)
                expect(disk).toBe('first disk')
                expect(deferredSave).toEqual([])
                await click('Unbind second')
                await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(1)
                await click('Unbind first')
                await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(0)
                expect((await state()).documents).toBe(1)
                await click('Bind first')
                await expect.poll(async () => (await state()).views, { timeout: DEADLINE_MS }).toBe(1)
                expect((await state()).text).toBe('first later disk')
                await click('Save prepared')
                await expect.poll(async () => (await state()).saves, { timeout: DEADLINE_MS }).toEqual([true, false])
                await expect
                    .poll(() => requests.filter((request) => request.command === 'file_open').length, { timeout: DEADLINE_MS })
                    .toBe(FINAL_READS)
                await expect.poll(async () => (await state()).loaded, { timeout: DEADLINE_MS }).toBe(FINAL_LOADS)
                expect(disk).toBe('first later disk')
                expect((await state()).dirty).toBe(false)
                await click('Unbind first')
                await expect.poll(async () => (await state()).documents, { timeout: DEADLINE_MS }).toBe(0)
                await click('Dispose')
                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                const disposed = await state()
                const finalRequests = requests.length
                await page.waitForTimeout(QUIET_MS)
                expect(requests).toHaveLength(finalRequests)
                expect((await state()).wakes).toBe(disposed.wakes)
                expect(upgrades).toBe(FINAL_UPGRADES)
                expect(requests.filter((request) => request.command === 'file_save')).toHaveLength(FINAL_SAVES)
                expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                expect((await state()).responses).toEqual([])
                await click('Drop')
                expect(errors).toEqual([])
                const result = { status: 'passed', boot, saved, recovered, disposed, upgrades, active, requests, errors }
                await Bun.write(resolve(fixture, 'file-result.json'), JSON.stringify(result, null, 2))
                console.info(JSON.stringify(result))
            } finally {
                await context.close()
            }
        } finally {
            await browser.close()
        }
    } finally {
        await server.stop(true)
    }
}
await verify()
