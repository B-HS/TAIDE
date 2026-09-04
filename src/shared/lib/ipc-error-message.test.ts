import { describe, expect, test } from 'bun:test'
import { applyLocaleMessages, i18next } from '@shared/i18n/i18n'
import { IpcError } from '@shared/api/unwrap-result'
import { describeIpcError, isIpcErrorKey, isNotFoundIpcError } from '@shared/lib/ipc-error-message'

const REGISTERED_KEY = 'error.test.ipcErrorMessageKnown'
const UNREGISTERED_KEY = 'error.test.ipcErrorMessageMissing'

applyLocaleMessages(i18next.language, { [REGISTERED_KEY]: 'known: {{name}}' })

const buildLocalizedIpcError = (key: string, fallback: string) =>
    new IpcError({ code: 'Localized', message: { kind: 'Internal', key, args: { name: 'foo' }, fallback } })

class FakeLocaleKeyError extends Error {
    readonly localeKey: string | undefined
    readonly localeArgs: Record<string, string> | undefined

    constructor(message: string, localeKey: string, localeArgs: Record<string, string>) {
        super(message)
        this.localeKey = localeKey
        this.localeArgs = localeArgs
    }
}

describe('describeIpcError', () => {
    test('localeKey 가 카탈로그에 있으면 번역된 문자열을 인자와 함께 반환한다', () => {
        const error = buildLocalizedIpcError(REGISTERED_KEY, 'fallback text')
        expect(describeIpcError(error)).toBe('known: foo')
    })

    test('IpcError 가 아니어도 localeKey 를 가진 Error 는 번역된다 (구조 판정 게이트 — RawFileReadError 등)', () => {
        const error = new FakeLocaleKeyError('fallback text', REGISTERED_KEY, { name: 'foo' })
        expect(describeIpcError(error)).toBe('known: foo')
    })

    test('localeKey 가 카탈로그에 없으면 fallback 문자열을 반환한다 (키 문자열 노출 방지)', () => {
        const error = buildLocalizedIpcError(UNREGISTERED_KEY, 'fallback text')
        expect(describeIpcError(error)).toBe('fallback text')
    })

    test('비-Localized AppError 는 message 를 그대로 반환한다', () => {
        const error = new IpcError({ code: 'NotFound', message: 'gone' })
        expect(describeIpcError(error)).toBe('gone')
    })

    test('IpcError 가 아닌 일반 Error 는 message 를 반환한다', () => {
        expect(describeIpcError(new Error('boom'))).toBe('boom')
    })

    test('Error 도 아닌 값은 문자열화해 반환한다', () => {
        expect(describeIpcError('plain string')).toBe('plain string')
    })
})

describe('isIpcErrorKey', () => {
    test('localeKey 가 일치하면 true 를 반환한다', () => {
        const error = buildLocalizedIpcError(REGISTERED_KEY, 'fallback text')
        expect(isIpcErrorKey(error, REGISTERED_KEY)).toBe(true)
    })

    test('localeKey 가 다르면 false 를 반환한다', () => {
        const error = buildLocalizedIpcError(REGISTERED_KEY, 'fallback text')
        expect(isIpcErrorKey(error, UNREGISTERED_KEY)).toBe(false)
    })

    test('IpcError 가 아니면 항상 false 를 반환한다', () => {
        expect(isIpcErrorKey(new Error('boom'), REGISTERED_KEY)).toBe(false)
    })
})

describe('isNotFoundIpcError', () => {
    test('NotFound 는 맨 AppError 든 Localized 로 감싸인 것이든 true 를 반환한다', () => {
        expect(isNotFoundIpcError(new IpcError({ code: 'NotFound', message: 'gone' }))).toBe(true)
        expect(
            isNotFoundIpcError(
                new IpcError({ code: 'Localized', message: { kind: 'NotFound', key: 'error.file.notFound', args: {}, fallback: 'gone' } }),
            ),
        ).toBe(true)
    })

    test('다른 코드거나 IpcError 가 아니면 false 를 반환한다', () => {
        expect(isNotFoundIpcError(new IpcError({ code: 'Internal', message: 'boom' }))).toBe(false)
        expect(isNotFoundIpcError(new Error('boom'))).toBe(false)
    })

    test('Forbidden·Io·InvalidArgument 는 맨 AppError 든 Localized 로 감싸인 것이든 false 를 반환한다 (useOpenFileTab 이 인덱스를 다시 걷지 않는 실패)', () => {
        expect(isNotFoundIpcError(new IpcError({ code: 'Forbidden', message: 'outside project' }))).toBe(false)
        expect(isNotFoundIpcError(new IpcError({ code: 'Io', message: 'permission denied' }))).toBe(false)
        expect(isNotFoundIpcError(new IpcError({ code: 'InvalidArgument', message: 'bad path' }))).toBe(false)
        expect(
            isNotFoundIpcError(
                new IpcError({
                    code: 'Localized',
                    message: { kind: 'Forbidden', key: 'error.path.outsideOpenProjects', args: { path: '/etc/passwd' }, fallback: 'outside' },
                }),
            ),
        ).toBe(false)
    })

    test('message 가 NotFound 처럼 보여도 코드가 다르면 false 이고, Error 가 아닌 값은 항상 false 다', () => {
        expect(isNotFoundIpcError(new IpcError({ code: 'Internal', message: 'NotFound' }))).toBe(false)
        expect(isNotFoundIpcError(new FakeLocaleKeyError('gone', 'error.file.notFound', {}))).toBe(false)
        expect(isNotFoundIpcError('NotFound')).toBe(false)
        expect(isNotFoundIpcError(null)).toBe(false)
        expect(isNotFoundIpcError(undefined)).toBe(false)
    })
})
