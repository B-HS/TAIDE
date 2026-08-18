import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { E2E_AUTH_DIR, RUNTIME_INFO_PATH } from './paths'

export type RuntimeInfo = {
    port: number
    baseURL: string
}

export const writeRuntimeInfo = async (info: RuntimeInfo) => {
    await mkdir(E2E_AUTH_DIR, { recursive: true })
    await writeFile(RUNTIME_INFO_PATH, JSON.stringify(info), 'utf8')
}

/**
 * Reads the runtime info `globalSetup` persisted after a fully successful bootstrap (port
 * discovery + login + storageState). Its absence means bootstrap never completed, which fixtures
 * treat as a fail-fast condition rather than attempting their own rediscovery.
 */
export const readRuntimeInfo = async () => {
    const raw = await readFile(RUNTIME_INFO_PATH, 'utf8').catch((error: unknown) => {
        void error
        return null
    })
    if (raw === null) return null
    return JSON.parse(raw) as RuntimeInfo
}
