import { describe, expect, test } from 'bun:test'
import { IpcError, unwrapResult } from '@shared/api/unwrap-result'

describe('unwrapResult', () => {
    test('ok 결과는 data 를 반환한다', async () => {
        await expect(unwrapResult(Promise.resolve({ status: 'ok' as const, data: 7 }))).resolves.toBe(7)
    })

    test('error 결과는 IpcError 로 throw 한다', async () => {
        const pending = unwrapResult(Promise.resolve({ status: 'error' as const, error: { code: 'NotFound' as const, message: 'gone' } }))
        await expect(pending).rejects.toBeInstanceOf(IpcError)
    })

    test('IpcError 는 code 와 message 를 보존한다', async () => {
        try {
            await unwrapResult(Promise.resolve({ status: 'error' as const, error: { code: 'Internal' as const, message: 'boom' } }))
            throw new Error('unreachable')
        } catch (error) {
            expect(error).toBeInstanceOf(IpcError)
            expect((error as IpcError).code).toBe('Internal')
            expect((error as IpcError).message).toBe('boom')
        }
    })
})
