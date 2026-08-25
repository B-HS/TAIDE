import { invoke } from '@tauri-apps/api/core'
import type { AppError, AppErrorKind } from '@shared/api/bindings'
import { isAppError, normalizeAppError } from '@shared/api/unwrap-result'

export class RawFileReadError extends Error {
    readonly code: AppErrorKind
    readonly localeKey: string | undefined
    readonly localeArgs: Record<string, string> | undefined

    constructor(error: AppError) {
        const normalized = normalizeAppError(error)
        super(normalized.message)
        this.name = 'RawFileReadError'
        this.code = normalized.code
        this.localeKey = normalized.localeKey
        this.localeArgs = normalized.localeArgs
    }
}

export const readFileRaw = async (path: string) => {
    try {
        return await invoke<ArrayBuffer>('file_read_raw', { path })
    } catch (error) {
        if (isAppError(error)) throw new RawFileReadError(error)
        throw error instanceof Error ? error : new Error(String(error))
    }
}
