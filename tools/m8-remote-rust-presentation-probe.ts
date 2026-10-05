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
const FINAL_UPGRADES = 2
const COALESCED_THEME_READS = 3
const FINAL_THEME_READS = 5
const FINAL_LOCALE_READS = 6
const COLOR = [16, 32, 48, 255]
const fixture = process.argv[ARG_INDEX]
if (!fixture || !/^\/private\/tmp\/taide-m8-wasm-tools\.[A-Za-z0-9]+\/presentation-built$/.test(fixture)) {
    throw new Error('Use the dedicated generated presentation probe directory')
}
const mode = z.enum(['continuous', 'unsupported-media']).parse(process.argv[MODE_INDEX] ?? 'continuous')
const isMediaUnsupported = mode === 'unsupported-media'
const requestSchema = z.object({ seq: z.number().int().nonnegative(), command: z.string(), args: z.record(z.string(), z.unknown()).nullable() })
const settingsSchema = z.object({ language: z.string(), editorFontSize: z.number() }).passthrough()
const stateSchema = z.object({
    connected: z.boolean(),
    ready: z.boolean(),
    systemTheme: z.enum(['dark', 'light']),
    systemLanguage: z.string(),
    theme: z.string().nullable(),
    locale: z.string().nullable(),
    message: z.string().nullable(),
    background: z.array(z.number()).nullable(),
    editorFont: z.number().nullable(),
    failures: z.number().int().nonnegative(),
    refreshing: z.boolean(),
    recoveries: z.number().int().nonnegative(),
    responses: z.array(z.number().int().nonnegative()),
    themes: z.array(z.string()),
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
]
let settings: z.infer<typeof settingsSchema> | null = null
let upgrades = 0
let active = 0
let isThemeHeld = false
let isLocaleHeld = false
let shouldFailLocale = false
let themeRevision = 1
let deferredTheme = Array<() => void>()
let deferredLocale = Array<() => void>()
let requests = Array<z.infer<typeof requestSchema>>()
const server = Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch: async (request, server) => {
        const url = new URL(request.url)
        if (url.pathname === '/__taide/ws') {
            upgrades += 1
            if (server.upgrade(request)) return
            return new Response('Synthetic upgrade failed', { status: HTTP_BAD_REQUEST })
        }
        if (url.pathname === '/__fixture/settings' && request.method === 'POST') {
            settings = settingsSchema.parse(await request.json())
            return new Response(null)
        }
        if (url.pathname === '/__fixture/control' && request.method === 'POST') {
            const command = z
                .enum(['hold-theme', 'release-theme', 'theme-event', 'hold-locale', 'change-language', 'fail-locale'])
                .parse(await request.json())
            if (command === 'hold-theme') isThemeHeld = true
            if (command === 'release-theme') {
                isThemeHeld = false
                for (const release of deferredTheme) release()
                deferredTheme = []
            }
            if (command === 'theme-event') {
                themeRevision += 1
                server.publish('fixture', JSON.stringify({ t: 'event', event: 'theme:changed', payload: JSON.stringify({ themeId: 'synthetic' }) }))
            }
            if (command === 'hold-locale') isLocaleHeld = true
            if (command === 'change-language') {
                if (!settings) throw new Error('Synthetic settings are missing')
                settings = { ...settings, language: 'ja' }
                server.publish('fixture', JSON.stringify({ t: 'event', event: 'settings:changed', payload: JSON.stringify({ settings }) }))
                isLocaleHeld = false
                for (const release of deferredLocale) release()
                deferredLocale = []
            }
            if (command === 'fail-locale') shouldFailLocale = true
            return new Response(null)
        }
        if (url.pathname === '/') return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe/presentation.html')))
        const name = url.pathname.slice(1)
        if (name === 'probe.js' || name === 'probe_bg.wasm') return new Response(Bun.file(resolve(fixture, name)))
        return new Response(null, { status: HTTP_NOT_FOUND })
    },
    websocket: {
        open: (socket) => {
            active += 1
            socket.subscribe('fixture')
        },
        close: () => {
            active -= 1
        },
        message: (socket, message) => {
            if (typeof message !== 'string') throw new Error('Only synthetic JSON requests are accepted')
            const request = requestSchema.parse(JSON.parse(message))
            requests = [...requests, request]
            if (!settings) throw new Error('Synthetic settings are missing')
            if (request.command === 'synthetic_cut_connection') {
                socket.close(RESTART_CODE, 'synthetic restart')
                return
            }
            if (request.command === 'theme_get_current') {
                const args = z
                    .object({ systemTheme: z.enum(['dark', 'light']) })
                    .strict()
                    .parse(request.args)
                const payload = {
                    id: `${themeRevision}-${args.systemTheme}`,
                    name: 'Synthetic',
                    type: args.systemTheme,
                    colors: Object.fromEntries(colorKeys.map((key) => [key, '#102030'])),
                    syntax: {},
                    terminal: {},
                }
                const response = JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload })
                if (isThemeHeld) {
                    deferredTheme = [...deferredTheme, () => socket.send(response)]
                    return
                }
                socket.send(response)
                return
            }
            if (request.command === 'locale_get_current') {
                expect(request.args).toEqual({ systemLanguage: 'ko-KR' })
                if (shouldFailLocale) {
                    shouldFailLocale = false
                    socket.send(
                        JSON.stringify({
                            t: 'resp',
                            seq: request.seq,
                            ok: false,
                            payload: { code: 'INTERNAL', message: 'synthetic locale failure' },
                        }),
                    )
                    return
                }
                const locale = settings.language === 'ja' ? 'ja' : 'ko'
                const response = JSON.stringify({
                    t: 'resp',
                    seq: request.seq,
                    ok: true,
                    payload: { id: locale, name: locale, messages: { hello: `${locale} {{name}}` } },
                })
                if (isLocaleHeld) {
                    deferredLocale = [...deferredLocale, () => socket.send(response)]
                    return
                }
                socket.send(response)
                return
            }
            const payloads = {
                project_list: [],
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
            const context = await browser.newContext({ locale: 'ko-KR', colorScheme: 'light', serviceWorkers: 'block', acceptDownloads: false })
            try {
                const page = await context.newPage()
                if (isMediaUnsupported) {
                    await page.addInitScript(() => {
                        Object.defineProperty(window, 'matchMedia', { value: undefined })
                    })
                }
                let errors = Array<string>()
                page.on('pageerror', (error) => {
                    errors = [...errors, error.message]
                })
                const readState = async () => stateSchema.parse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                const control = async (command: string) => {
                    const response = await context.request.post(`${server.url.origin}/__fixture/control`, { data: JSON.stringify(command) })
                    expect(response.ok()).toBe(true)
                }
                await page.goto(server.url.href)
                await expect
                    .poll(
                        async () => {
                            const parsed = stateSchema.safeParse(JSON.parse((await page.locator('#state').textContent()) ?? '{}'))
                            return parsed.success && parsed.data.ready
                        },
                        { timeout: DEADLINE_MS },
                    )
                    .toBe(true)
                const boot = await readState()
                const bootTheme = isMediaUnsupported ? 'dark' : 'light'
                expect(boot.systemTheme).toBe(bootTheme)
                expect(boot.systemLanguage).toBe('ko-KR')
                expect(boot.theme).toBe(`1-${bootTheme}`)
                expect(boot.locale).toBe('ko')
                expect(boot.message).toBe('ko Rust')
                expect(boot.background).toEqual(COLOR)
                expect(boot.failures).toBe(0)
                expect(boot.editorFont).toBe(settingsSchema.parse(settings).editorFontSize)
                expect(requests.map((request) => request.command)).toEqual([
                    'project_list',
                    'project_group_list',
                    'session_get_shell_state',
                    'settings_get',
                    'theme_get_current',
                    'locale_get_current',
                ])
                expect(boot.responses).toEqual([])

                if (isMediaUnsupported) {
                    await page.getByRole('button', { name: 'Dispose', exact: true }).click()
                    await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                    const finalRequests = requests.length
                    await page.waitForTimeout(QUIET_MS)
                    expect(requests).toHaveLength(finalRequests)
                    expect(upgrades).toBe(1)
                    await page.getByRole('button', { name: 'Drop', exact: true }).click()
                    expect(errors).toEqual([])
                    const result = { status: 'passed', mode, boot, upgrades, active, requests, errors }
                    await Bun.write(resolve(fixture, 'presentation-unsupported-result.json'), JSON.stringify(result, null, 2))
                    console.info(JSON.stringify(result))
                    return
                }

                await control('hold-theme')
                await page.emulateMedia({ colorScheme: 'dark' })
                await expect.poll(() => deferredTheme.length, { timeout: DEADLINE_MS }).toBe(1)
                const heldTheme = requests.at(-1)
                expect(heldTheme?.args).toEqual({ systemTheme: 'dark' })
                await page.emulateMedia({ colorScheme: 'light' })
                await expect.poll(async () => (await readState()).systemTheme, { timeout: DEADLINE_MS }).toBe('light')
                await control('release-theme')
                await expect
                    .poll(() => requests.filter((request) => request.command === 'theme_get_current').length, { timeout: DEADLINE_MS })
                    .toBe(COALESCED_THEME_READS)
                await control('theme-event')
                await expect.poll(async () => (await readState()).theme, { timeout: DEADLINE_MS }).toBe('2-light')
                expect((await readState()).themes).toEqual(['1-light', '2-light'])

                await control('hold-locale')
                await page.getByRole('button', { name: 'Retry locale', exact: true }).click()
                await expect.poll(() => deferredLocale.length, { timeout: DEADLINE_MS }).toBe(1)
                await control('change-language')
                await expect.poll(async () => (await readState()).locale, { timeout: DEADLINE_MS }).toBe('ja')
                expect((await readState()).message).toBe('ja Rust')
                await control('fail-locale')
                await page.getByRole('button', { name: 'Retry locale', exact: true }).click()
                await expect.poll(async () => (await readState()).failures, { timeout: DEADLINE_MS }).toBe(1)
                expect((await readState()).locale).toBe('ja')
                await page.getByRole('button', { name: 'Retry locale', exact: true }).click()
                await expect.poll(async () => (await readState()).failures, { timeout: DEADLINE_MS }).toBe(0)

                await page.getByRole('button', { name: 'Reconnect', exact: true }).click()
                await expect
                    .poll(
                        async () => {
                            const state = await readState()
                            return (
                                state.connected &&
                                state.recoveries === 1 &&
                                !state.refreshing &&
                                state.responses.length === 0 &&
                                requests.filter((request) => request.command === 'theme_get_current').length === FINAL_THEME_READS &&
                                requests.filter((request) => request.command === 'locale_get_current').length === FINAL_LOCALE_READS
                            )
                        },
                        { timeout: DEADLINE_MS },
                    )
                    .toBe(true)
                const recovered = await readState()
                expect(recovered.theme).toBe('2-light')
                expect(recovered.locale).toBe('ja')
                expect(recovered.failures).toBe(0)
                expect(upgrades).toBe(FINAL_UPGRADES)
                expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
                await page.getByRole('button', { name: 'Dispose', exact: true }).click()
                await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
                await expect.poll(async () => (await readState()).connected, { timeout: DEADLINE_MS }).toBe(false)
                const disposed = await readState()
                const finalRequests = requests.length
                await page.emulateMedia({ colorScheme: 'dark' })
                await page.waitForTimeout(QUIET_MS)
                expect((await readState()).wakes).toBe(disposed.wakes)
                expect(requests).toHaveLength(finalRequests)
                expect(upgrades).toBe(FINAL_UPGRADES)
                await page.getByRole('button', { name: 'Drop', exact: true }).click()
                expect(errors).toEqual([])
                const result = { status: 'passed', boot, recovered, disposed, upgrades, active, requests, errors }
                await Bun.write(resolve(fixture, 'presentation-result.json'), JSON.stringify(result, null, 2))
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
