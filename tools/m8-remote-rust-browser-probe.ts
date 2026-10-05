import { chromium, expect } from '@playwright/test'
import { z } from 'zod'
import { resolve } from 'node:path'

const CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const DEADLINE_MS = 10_000
const QUIET_MS = 1_100
const FIRST_ACCEPTED_UPGRADE = 2
const RECOVERY_COUNT = 2
const UPGRADE_COUNT_BEFORE_EXPIRATION = 3
const CHANNEL_ID = 17
const CHANNEL_END_INDEX = 2
const RESPONSE_TAG = 2
const CHANNEL_TAG = 1
const RESPONSE_HEADER_BYTES = 5
const CHANNEL_HEADER_BYTES = 9
const SEQUENCE_OFFSET = 1
const INDEX_OFFSET = 5
const RESTART_CODE = 1012
const EXPIRED_CODE = 4001
const HTTP_UNAVAILABLE = 503
const HTTP_BAD_REQUEST = 400
const HTTP_NOT_FOUND = 404
const PAYLOAD = Uint8Array.of(0, 255, 12)
const fixture = process.argv[2]
if (!fixture || !/^\/private\/tmp\/taide-m8-wasm-tools\.[A-Za-z0-9]+\/built$/.test(fixture)) {
    throw new Error('Use the dedicated generated probe directory')
}
const requestSchema = z.object({ seq: z.number().int().nonnegative(), command: z.string(), args: z.null() })
const eventsSchema = z.array(
    z.object({
        kind: z.string(),
        seq: z.number().int().nonnegative().optional(),
        recovered: z.boolean().optional(),
        rejected: z.array(z.number().int().nonnegative()).optional(),
        channelId: z.number().int().nonnegative().optional(),
        index: z.number().int().nonnegative().optional(),
        event: z.string().optional(),
        payload: z.union([z.array(z.number()), z.object({ synthetic: z.boolean() }), z.string(), z.null()]).optional(),
    }),
)
let upgrades = 0
let active = 0
let requests = Array<z.infer<typeof requestSchema>>()
const server = Bun.serve({
    hostname: '127.0.0.1',
    port: 0,
    fetch: async (request, server) => {
        const url = new URL(request.url)
        if (url.pathname === '/__taide/ws') {
            upgrades += 1
            if (upgrades < FIRST_ACCEPTED_UPGRADE) return new Response('Synthetic handshake failure', { status: HTTP_UNAVAILABLE })
            if (server.upgrade(request)) return
            return new Response('Synthetic upgrade failed', { status: HTTP_BAD_REQUEST })
        }
        if (url.pathname === '/__taide/login') return new Response('Synthetic login', { headers: { 'content-type': 'text/html' } })
        if (url.pathname === '/') {
            return new Response(Bun.file(resolve('native/taide-remote-web/tests/browser-probe/index.html')))
        }
        const name = url.pathname.slice(1)
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
            if (typeof message !== 'string') throw new Error('Only synthetic JSON requests are accepted')
            const request = requestSchema.parse(JSON.parse(message))
            requests = [...requests, request]
            if (request.command === 'cut_connection') {
                socket.close(RESTART_CODE, 'synthetic restart')
                return
            }
            if (request.command === 'expire') {
                socket.close(EXPIRED_CODE, 'synthetic expiration')
                return
            }
            if (request.command === 'file_read_raw') {
                const frame = new Uint8Array(RESPONSE_HEADER_BYTES + PAYLOAD.length)
                frame[0] = RESPONSE_TAG
                new DataView(frame.buffer).setUint32(SEQUENCE_OFFSET, request.seq)
                frame.set(PAYLOAD, RESPONSE_HEADER_BYTES)
                socket.send(frame)
                return
            }
            if (request.command === 'subscribe') {
                socket.send(JSON.stringify({ t: 'chan', channelId: CHANNEL_ID, index: 0, message: { synthetic: true } }))
                const frame = new Uint8Array(CHANNEL_HEADER_BYTES + PAYLOAD.length)
                frame[0] = CHANNEL_TAG
                const header = new DataView(frame.buffer)
                header.setUint32(SEQUENCE_OFFSET, CHANNEL_ID)
                header.setUint32(INDEX_OFFSET, 1)
                frame.set(PAYLOAD, CHANNEL_HEADER_BYTES)
                socket.send(frame)
                socket.send(JSON.stringify({ t: 'chanEnd', channelId: CHANNEL_ID, index: CHANNEL_END_INDEX }))
                socket.send(JSON.stringify({ t: 'event', event: 'synthetic', payload: JSON.stringify({ synthetic: true }) }))
            }
            socket.send(JSON.stringify({ t: 'resp', seq: request.seq, ok: true, payload: { synthetic: true } }))
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
            const readEvents = async () => eventsSchema.parse(JSON.parse((await page.locator('#events').textContent()) ?? '[]'))
            const invoke = async (command: string) => {
                await page.locator('#command').fill(command)
                await page.getByRole('button', { name: 'Invoke', exact: true }).click()
            }
            const waitResponse = async (seq: number, kind = 'response') => {
                await expect
                    .poll(async () => (await readEvents()).some((event) => event.kind === kind && event.seq === seq), { timeout: DEADLINE_MS })
                    .toBe(true)
            }
            await page.goto(server.url.href)
            await expect
                .poll(async () => (await readEvents()).some((event) => event.kind === 'disconnected' && event.rejected?.includes(1)), {
                    timeout: DEADLINE_MS,
                })
                .toBe(true)
            expect(requests).toEqual([])
            await invoke('settings_get')
            const SETTINGS_SEQ = 2
            const RAW_SEQ = 3
            const SUBSCRIBE_SEQ = 4
            const CLOSED_SEQ = 5
            const RECONNECTED_SEQ = 6
            await waitResponse(SETTINGS_SEQ)
            await invoke('file_read_raw')
            await waitResponse(RAW_SEQ, 'binary')
            await invoke('subscribe')
            await waitResponse(SUBSCRIBE_SEQ)
            const frames = await readEvents()
            expect(frames.find((event) => event.kind === 'binary')?.payload).toEqual(Array.from(PAYLOAD))
            expect(frames.filter((event) => event.kind.startsWith('channel-')).map((event) => [event.kind, event.channelId, event.index])).toEqual([
                ['channel-json', CHANNEL_ID, 0],
                ['channel-binary', CHANNEL_ID, 1],
                ['channel-end', CHANNEL_ID, CHANNEL_END_INDEX],
            ])
            expect(frames.find((event) => event.kind === 'event')?.payload).toBe(JSON.stringify({ synthetic: true }))
            await invoke('cut_connection')
            await expect
                .poll(async () => (await readEvents()).some((event) => event.kind === 'disconnected' && event.rejected?.includes(CLOSED_SEQ)), {
                    timeout: DEADLINE_MS,
                })
                .toBe(true)
            await invoke('settings_get')
            await waitResponse(RECONNECTED_SEQ)
            expect((await readEvents()).filter((event) => event.kind === 'connected' && event.recovered)).toHaveLength(RECOVERY_COUNT)
            expect(requests.map((request) => request.command)).toEqual([
                'settings_get',
                'file_read_raw',
                'subscribe',
                'cut_connection',
                'settings_get',
            ])
            const wakes = Number(await page.locator('#wakes').textContent())
            expect(wakes).toBeGreaterThan(0)
            const completedFrames = await readEvents()
            await page.getByRole('button', { name: 'Dispose', exact: true }).click()
            await expect.poll(() => active, { timeout: DEADLINE_MS }).toBe(0)
            await Bun.sleep(QUIET_MS)
            expect(upgrades).toBe(UPGRADE_COUNT_BEFORE_EXPIRATION)
            await page.getByRole('button', { name: 'Recreate', exact: true }).click()
            await invoke('expire')
            await page.waitForURL(`${server.url.origin}/__taide/login`, { timeout: DEADLINE_MS })
            await Bun.sleep(QUIET_MS)
            expect(upgrades).toBe(UPGRADE_COUNT_BEFORE_EXPIRATION + 1)
            expect(active).toBe(0)
            expect(errors).toEqual([])
            const result = { status: 'passed', upgrades, active, requests, wakes, events: completedFrames }
            await Bun.write(resolve(fixture, 'result.json'), JSON.stringify(result))
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
