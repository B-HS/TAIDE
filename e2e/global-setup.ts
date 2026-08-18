import { request, webkit } from '@playwright/test'
import { mkdir, writeFile } from 'node:fs/promises'
import { discoverPort } from './lib/discover-port'
import { invokeIpc } from './lib/ipc'
import { describeLoginFailure, loginOnce } from './lib/login'
import { E2E_AUTH_DIR, STORAGE_STATE_PATH, SETTINGS_SNAPSHOT_PATH } from './lib/paths'
import { writeRuntimeInfo } from './lib/runtime-info'

/**
 * Runs once before the whole suite. Never starts the app — only connects to whatever
 * remote-control server `bun run tauri dev` already left listening on loopback.
 *
 * Order matters: port discovery, then a single login attempt (never retried — see `login.ts`),
 * then a settings snapshot taken through a real webkit page (needed because `settings_get` is a
 * Tauri IPC call, reachable only from a loaded app page, not from `request`'s HTTP-only context).
 * `runtime.json` is written last, so its mere presence tells fixtures the whole bootstrap
 * succeeded — see `lib/taide-fixture.ts`.
 */
const globalSetup = async () => {
    await mkdir(E2E_AUTH_DIR, { recursive: true })

    const port = await discoverPort()
    const baseURL = `http://127.0.0.1:${port}`

    const requestContext = await request.newContext({ baseURL })
    try {
        const outcome = await loginOnce(requestContext, baseURL)
        if (outcome.kind !== 'success') throw new Error(describeLoginFailure(outcome))
        await requestContext.storageState({ path: STORAGE_STATE_PATH })
    } finally {
        await requestContext.dispose()
    }

    const browser = await webkit.launch()
    try {
        const context = await browser.newContext({ storageState: STORAGE_STATE_PATH })
        const page = await context.newPage()
        await page.goto(baseURL)
        const settingsSnapshot = await invokeIpc(page, 'settings_get')
        await writeFile(SETTINGS_SNAPSHOT_PATH, JSON.stringify(settingsSnapshot), 'utf8')
    } finally {
        await browser.close()
    }

    await writeRuntimeInfo({ port, baseURL })
}

export default globalSetup
