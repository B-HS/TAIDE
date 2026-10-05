import { chromium, expect } from '@playwright/test'
import { z } from 'zod'
import { resolve } from 'node:path'

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const DEADLINE_MS = 10_000
const QUIET_MS = 1_100
const RESTART_CODE = 1012
const HTTP_BAD_REQUEST = 400
const HTTP_NOT_FOUND = 404
const FIRST_REVISION = 1
const LAYOUT_SCHEMA_VERSION = 2
const PINNED_REVISION = 2
const FINAL_UPGRADES = 2
const EXPECTED_COMMANDS = [
    'project_list',
    'project_group_list',
    'session_get_shell_state',
    'settings_get',
    'layout_get',
    'layout_pin_tab',
    'layout_get',
    'project_list',
    'project_group_list',
    'session_get_shell_state',
    'settings_get',
    'layout_get',
    'synthetic_cut_connection',
    'project_list',
    'project_group_list',
    'session_get_shell_state',
    'settings_get',
    'layout_get',
]
const ARG_INDEX = 2
const PROJECT = 'prj-synthetic'
const PANE = 'pane-synthetic'
const SLOT = 'slot-synthetic'
const TAB = 'tab-synthetic'
const fixture = process.argv[ARG_INDEX]
if (!fixture || !/^\/private\/tmp\/taide-m8-wasm-tools\.[A-Za-z0-9]+\/shell-built$/.test(fixture)) {
    throw new Error('Use the dedicated generated shell probe directory')
}
const requestSchema = z.object({ seq: z.number().int().nonnegative(), command: z.string(), args: z.record(z.string(), z.unknown()).nullable() })
const settingsSchema = z.object({ zenHideStatusBar: z.boolean(), resizerThickness: z.number() }).passthrough()
const stateSchema = z.object({
    connected: z.boolean(),
    ready: z.boolean(),
    projects: z.array(z.string()).nullable(),
    layoutCount: z.number().int().nonnegative().nullable(),
    focused: z.string().nullable(),
    revision: z.number().int().nonnegative().nullable(),
    pinned: z.boolean().nullable(),
    hideStatus: z.boolean().nullable(),
    failures: z.number().int().nonnegative(),
    recoveries: z.number().int().nonnegative(),
    responses: z.array(z.number().int().nonnegative()),
    wakes: z.number().int().nonnegative(),
    uuid: z.string(),
})
let settings: z.infer<typeof settingsSchema> | null = null
let upgrades = 0
let active = 0
let hasProject = true
let isSettingsHeld = false
let deferredSettings = Array<() => void>()
let requests = Array<z.infer<typeof requestSchema>>()
let layout = {
    version: LAYOUT_SCHEMA_VERSION,
    revision: FIRST_REVISION,
    root: {
        node: 'leaf',
        id: PANE,
        tabs: [{ id: TAB, kind: { kind: 'welcome' }, title: 'Synthetic', pinned: false, preview: false, dirty: false, viewState: null }],
        active: TAB,
    },
    focusedPane: PANE,
}
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
            const command = z.enum(['hold-settings', 'release-settings']).parse(await request.json())
            if (command === 'hold-settings') {
                isSettingsHeld = true
                return new Response(null)
            }
            if (!settings) throw new Error('Synthetic settings are missing')
            settings = { ...settings, zenHideStatusBar: !settings.zenHideStatusBar }
            server.publish('fixture', JSON.stringify({ t: 'event', event: 'settings:changed', payload: JSON.stringify({ settings }) }))
            for (const release of deferredSettings) release()
            deferredSettings = []
            isSettingsHeld = false
            return new Response(null)
        }
        if (url.pathname === '/') return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe/shell.html')))
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
            if (request.command === 'synthetic_cut_connection') {
                hasProject = false
                socket.close(RESTART_CODE, 'synthetic restart')
                return
            }
            if (request.command === 'layout_pin_tab') {
                expect(request.args).toEqual({ tabId: TAB, pinned: true })
                layout = {
                    ...layout,
                    revision: PINNED_REVISION,
                    root: { ...layout.root, tabs: layout.root.tabs.map((tab) => ({ ...tab, pinned: true })) },
                }
                socket.send(
                    JSON.stringify({
                        t: 'event',
                        event: 'layout:changed',
                        payload: JSON.stringify({ projectId: PROJECT, revision: PINNED_REVISION }),
                    }),
                )
                socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: layout }))
                return
            }
            if (request.command === 'settings_get') {
                if (!settings) throw new Error('Synthetic settings are missing')
                const response = JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: settings })
                if (isSettingsHeld) {
                    deferredSettings = [...deferredSettings, () => socket.send(response)]
                    return
                }
                socket.send(response)
                return
            }
            const payloads = {
                project_list: hasProject ? [{ id: PROJECT, root: '/synthetic', name: 'Synthetic' }] : [],
                project_group_list: [],
                session_get_shell_state: {
                    tree: hasProject ? { node: 'leaf', slotId: SLOT, projectId: PROJECT } : null,
                    focused: hasProject ? SLOT : null,
                    windowChrome: { zen: false, sidebarRailCollapsed: false },
                },
                layout_get: layout,
            }
            const command = z.enum(['project_list', 'project_group_list', 'session_get_shell_state', 'layout_get']).parse(request.command)
            if (command === 'layout_get') expect(request.args).toEqual({ projectId: PROJECT })
            if (command !== 'layout_get') expect(request.args).toBeNull()
            socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: payloads[command] }))
        },
    },
})
try {
    const browser = await chromium.launch({ executablePath: CHROME, headless: true, args: ['--use-mock-keychain'] })
    try {
        const context = await browser.newContext({ serviceWorkers: 'block', acceptDownloads: false })
        try {
            const page = await context.newPage()
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
                        const text = await page.locator('#state').textContent()
                        const parsed = stateSchema.safeParse(JSON.parse(text ?? '{}'))
                        return parsed.success && parsed.data.connected && parsed.data.ready
                    },
                    { timeout: DEADLINE_MS },
                )
                .toBe(true)
            const boot = await readState()
            expect(boot.projects).toEqual([PROJECT])
            expect(boot.layoutCount).toBe(1)
            expect(boot.focused).toBe(PROJECT)
            expect(boot.revision).toBe(FIRST_REVISION)
            expect(boot.pinned).toBe(false)
            expect(boot.failures).toBe(0)
            expect(boot.uuid).toMatch(/^prj-[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
            expect(requests.map((request) => request.command)).toEqual([
                'project_list',
                'project_group_list',
                'session_get_shell_state',
                'settings_get',
                'layout_get',
            ])

            await page.getByRole('button', { name: 'Pin', exact: true }).click()
            await expect.poll(async () => (await readState()).pinned, { timeout: DEADLINE_MS }).toBe(true)
            expect((await readState()).revision).toBe(PINNED_REVISION)
            expect(requests.filter((request) => request.command === 'layout_pin_tab')).toHaveLength(1)
            await control('hold-settings')
            await page.getByRole('button', { name: 'Refresh', exact: true }).click()
            await expect.poll(() => deferredSettings.length, { timeout: DEADLINE_MS }).toBe(1)
            await control('release-settings')
            await expect.poll(async () => (await readState()).hideStatus, { timeout: DEADLINE_MS }).toBe(!boot.hideStatus)
            const changed = await readState()
            expect(changed.failures).toBe(0)
            expect(changed.pinned).toBe(true)
            await page.getByRole('button', { name: 'Reconnect', exact: true }).click()
            await expect.poll(async () => (await readState()).connected, { timeout: DEADLINE_MS }).toBe(false)
            await expect.poll(async () => (await readState()).projects, { timeout: DEADLINE_MS }).toEqual([])
            const recovered = await readState()
            expect(recovered.connected).toBe(true)
            expect(recovered.ready).toBe(true)
            expect(recovered.recoveries).toBe(1)
            expect(recovered.focused).toBeNull()
            expect(recovered.layoutCount).toBe(0)
            expect(recovered.revision).toBeNull()
            expect(recovered.pinned).toBeNull()
            expect(recovered.hideStatus).toBe(changed.hideStatus)
            expect(recovered.failures).toBe(0)
            expect(recovered.wakes).toBeGreaterThan(0)
            expect(upgrades).toBe(FINAL_UPGRADES)
            expect(requests.map((request) => request.command)).toEqual(EXPECTED_COMMANDS)
            expect(new Set(requests.map((request) => request.seq)).size).toBe(requests.length)
            await page.getByRole('button', { name: 'Drop', exact: true }).click()
            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
            await Bun.sleep(QUIET_MS)
            expect(upgrades).toBe(FINAL_UPGRADES)
            expect(requests.map((request) => request.command)).toEqual(EXPECTED_COMMANDS)
            expect(errors).toEqual([])
            const result = { status: 'passed', upgrades, active, requests, boot, changed, recovered, errors }
            await Bun.write(resolve(fixture, 'shell-result.json'), JSON.stringify(result))
            process.stdout.write(JSON.stringify(result))
        } finally {
            await context.close()
        }
    } finally {
        await browser.close()
    }
} finally {
    await server.stop(true)
}
