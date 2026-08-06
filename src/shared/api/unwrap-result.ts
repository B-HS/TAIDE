import type { AppError } from '@shared/api/bindings'

type IpcResult<T> = { status: 'ok'; data: T } | { status: 'error'; error: AppError }

export class IpcError extends Error {
    readonly code: AppError['code']

    constructor(error: AppError) {
        super(error.message)
        this.name = 'IpcError'
        this.code = error.code
    }
}

export const unwrapResult = async <T>(pending: Promise<IpcResult<T>>) => {
    const result = await pending
    if (result.status === 'error') throw new IpcError(result.error)
    return result.data
}
