import { invoke } from '@tauri-apps/api/core'
import type { AppError } from '@shared/api/bindings'

const isAppError = (value: unknown): value is AppError => typeof value === 'object' && value !== null && 'code' in value && 'message' in value

export class RawFileReadError extends Error {
    readonly code: AppError['code']

    constructor(error: AppError) {
        super(error.message)
        this.name = 'RawFileReadError'
        this.code = error.code
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
