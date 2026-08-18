import { webkit } from '@playwright/test'
import { readFile } from 'node:fs/promises'
import { invokeIpc } from './lib/ipc'
import { STORAGE_STATE_PATH, SETTINGS_SNAPSHOT_PATH } from './lib/paths'
import { readRuntimeInfo } from './lib/runtime-info'
import { buildFullRestorePatch, type SettingsSnapshot } from './lib/settings'

/**
 * Runs once after the whole suite, restoring every setting field back to the value `globalSetup`
 * snapshotted before any test ran — the other half of the § 설정 복원 규약 isolation contract.
 * Silently no-ops when bootstrap never got far enough to leave a snapshot (nothing to restore).
 */
const globalTeardown = async () => {
    const runtimeInfo = await readRuntimeInfo()
    if (!runtimeInfo) return

    const snapshotRaw = await readFile(SETTINGS_SNAPSHOT_PATH, 'utf8').catch((error: unknown) => {
        void error
        return null
    })
    if (snapshotRaw === null) return
    const snapshot = JSON.parse(snapshotRaw) as SettingsSnapshot

    const browser = await webkit.launch()
    try {
        const context = await browser.newContext({ storageState: STORAGE_STATE_PATH })
        const page = await context.newPage()
        await page.goto(runtimeInfo.baseURL)
        await invokeIpc(page, 'settings_update', { patch: buildFullRestorePatch(snapshot) })
    } finally {
        await browser.close()
    }
}

export default globalTeardown
