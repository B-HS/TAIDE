import type { AppError, AppErrorKind } from '@shared/api/bindings'

type IpcResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError }

/** Narrows a raw `invoke` rejection to a wire-level `AppError` — the boundary guard `readFileRaw`/`spawnPty`/`attachPty` need because their `Channel`-carrying `invoke` calls bypass `unwrapResult`'s typed `IpcResult` envelope and reject with the backend's `AppError` shape directly. */
export const isAppError = (value: unknown): value is AppError => typeof value === 'object' && value !== null && 'code' in value && 'message' in value

/**
 * Flattens the wire-level `AppError` union (5 legacy variants plus the `Localized` newtype
 * variant, see `error.rs`) into one shape every `IpcError`/`RawFileReadError` consumer can read
 * without a `code === 'Localized'` check of their own. `code` stays one of the pre-taxonomy 5
 * `AppErrorKind` values either way — `Localized` itself is never exposed as a `code` — so the 4
 * existing `IpcError.code` branch sites keep working unmodified.
 */
export const normalizeAppError = (error: AppError) =>
    error.code === 'Localized'
        ? { code: error.message.kind, message: error.message.fallback, localeKey: error.message.key, localeArgs: error.message.args }
        : { code: error.code, message: error.message, localeKey: undefined, localeArgs: undefined }

export class IpcError extends Error {
    readonly code: AppErrorKind
    readonly localeKey: string | undefined
    readonly localeArgs: Record<string, string> | undefined

    constructor(error: AppError) {
        const normalized = normalizeAppError(error)
        super(normalized.message)
        this.name = 'IpcError'
        this.code = normalized.code
        this.localeKey = normalized.localeKey
        this.localeArgs = normalized.localeArgs
    }
}

export const unwrapResult = async <T>(pending: Promise<IpcResult<T>>) => {
    const result = await pending
    if (result.status === 'error') throw new IpcError(result.error)
    return result.data
}
