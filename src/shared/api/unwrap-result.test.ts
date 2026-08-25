import { describe, expect, test } from 'bun:test'
import { IpcError, isAppError, normalizeAppError, unwrapResult } from '@shared/api/unwrap-result'

describe('isAppError', () => {
    test('code 와 message 를 가진 객체는 AppError 로 판별한다', () => {
        expect(isAppError({ code: 'NotFound', message: 'gone' })).toBe(true)
    })

    test('code 또는 message 가 없는 값은 AppError 가 아니다', () => {
        expect(isAppError({ code: 'NotFound' })).toBe(false)
        expect(isAppError({ message: 'gone' })).toBe(false)
        expect(isAppError(null)).toBe(false)
        expect(isAppError('boom')).toBe(false)
    })
})

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

describe('normalizeAppError', () => {
    test('5변종(Localized 제외)은 code·message 를 그대로 통과시키고 localeKey/localeArgs 는 undefined 다', () => {
        expect(normalizeAppError({ code: 'Forbidden', message: 'nope' })).toEqual({
            code: 'Forbidden',
            message: 'nope',
            localeKey: undefined,
            localeArgs: undefined,
        })
    })

    test('Localized 변종은 kind 를 code 로, fallback 을 message 로 평탄화하고 key/args 를 노출한다', () => {
        const normalized = normalizeAppError({
            code: 'Localized',
            message: { kind: 'NotFound', key: 'error.git.noChanges', args: { path: 'src/a.rs' }, fallback: 'src/a.rs: no changes' },
        })

        expect(normalized).toEqual({
            code: 'NotFound',
            message: 'src/a.rs: no changes',
            localeKey: 'error.git.noChanges',
            localeArgs: { path: 'src/a.rs' },
        })
    })
})

describe('IpcError — Localized 변종', () => {
    test('code 는 Localized 를 노출하지 않고 kind 로 정규화된다 (기존 4곳의 code 분기 보존)', async () => {
        const pending = unwrapResult(
            Promise.resolve({
                status: 'error' as const,
                error: {
                    code: 'Localized' as const,
                    message: {
                        kind: 'InvalidArgument' as const,
                        key: 'error.lsp.installCancelled',
                        args: {},
                        fallback: 'Installation was cancelled',
                    },
                },
            }),
        )

        try {
            await pending
            throw new Error('unreachable')
        } catch (error) {
            expect(error).toBeInstanceOf(IpcError)
            const ipcError = error as IpcError
            expect(ipcError.code).toBe('InvalidArgument')
            expect(ipcError.message).toBe('Installation was cancelled')
            expect(ipcError.localeKey).toBe('error.lsp.installCancelled')
            expect(ipcError.localeArgs).toEqual({})
        }
    })
})
