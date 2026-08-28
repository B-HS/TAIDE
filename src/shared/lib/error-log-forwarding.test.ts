import { afterEach, beforeAll, describe, expect, mock, test } from 'bun:test'

/**
 * `error-log-forwarding.ts` imports `@tauri-apps/plugin-log` at module scope, so this file mocks it
 * before ever reaching the module under test — via a *dynamic* `import()`, not a static one, since
 * Bun resolves a file's whole static import graph (pulling in the real `@tauri-apps/plugin-log`)
 * before any of that file's own `mock.module` calls would run (same constraint
 * `terminal.ipc.test.ts` documents). `logErrorSink` is swapped per test to control what the mocked
 * `error()` export does — resolve, reject, or synchronously call back into the wrapped console.
 */
type LogSink = (message: string) => Promise<void>

const defaultLogSink: LogSink = () => Promise.resolve()
let logErrorSink: LogSink = defaultLogSink

const logErrorMock = mock((message: string) => logErrorSink(message))
const logWarnMock = mock((_message: string) => Promise.resolve())

mock.module('@tauri-apps/plugin-log', () => ({ error: logErrorMock, warn: logWarnMock }))

const importForwarding = () => import('@shared/lib/error-log-forwarding')

describe('serializeLogArgument', () => {
    test('Error 는 message 와 stack 을 함께 직렬화한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        const error = new Error('boom')

        const serialized = serializeLogArgument(error)

        expect(serialized.startsWith('boom')).toBe(true)
        expect(serialized).toContain(error.stack ?? '')
    })

    test('stack 이 없는 Error 는 message 만 직렬화한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        const error = new Error('boom')
        error.stack = undefined

        expect(serializeLogArgument(error)).toBe('boom')
    })

    test('문자열은 그대로 반환한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        expect(serializeLogArgument('plain message')).toBe('plain message')
    })

    test('일반 객체는 JSON.stringify 로 직렬화한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        expect(serializeLogArgument({ code: 'E_FAIL', count: 2 })).toBe('{"code":"E_FAIL","count":2}')
    })

    test('순환 참조 객체는 JSON.stringify 실패 시 String() 으로 폴백한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        const circular: Record<string, unknown> = {}
        circular.self = circular

        expect(serializeLogArgument(circular)).toBe(String(circular))
    })

    test('undefined 는 String() 결과로 직렬화한다', async () => {
        const { serializeLogArgument } = await importForwarding()
        expect(serializeLogArgument(undefined)).toBe('undefined')
    })
})

describe('serializeLogArguments', () => {
    test('여러 인자를 공백으로 이어 붙인다', async () => {
        const { serializeLogArguments } = await importForwarding()
        expect(serializeLogArguments(['failed:', { code: 404 }])).toBe('failed: {"code":404}')
    })

    test('빈 배열은 빈 문자열이다', async () => {
        const { serializeLogArguments } = await importForwarding()
        expect(serializeLogArguments([])).toBe('')
    })
})

describe('installErrorLogForwarding', () => {
    const originalConsoleError = mock((..._args: unknown[]) => {})
    const fakeConsole = { error: originalConsoleError, warn: mock((..._args: unknown[]) => {}) }

    beforeAll(async () => {
        const { installErrorLogForwarding } = await importForwarding()
        installErrorLogForwarding(fakeConsole)
    })

    afterEach(() => {
        logErrorSink = defaultLogSink
        logErrorMock.mockClear()
        originalConsoleError.mockClear()
    })

    test('후킹 후 원본 console.error 가 같은 인자로 먼저 호출된다', () => {
        const callOrder: string[] = []
        originalConsoleError.mockImplementationOnce(() => callOrder.push('original'))
        logErrorSink = (message) => {
            callOrder.push(`forwarded:${message}`)
            return Promise.resolve()
        }

        fakeConsole.error('boom', { code: 1 })

        expect(originalConsoleError).toHaveBeenCalledWith('boom', { code: 1 })
        expect(callOrder).toEqual(['original', 'forwarded:boom {"code":1}'])
    })

    test('포워딩 중 재진입해도 2차 포워딩은 발생하지 않는다', () => {
        logErrorSink = () => {
            fakeConsole.error('nested')
            return Promise.resolve()
        }

        fakeConsole.error('outer')

        expect(logErrorMock).toHaveBeenCalledTimes(1)
        expect(logErrorMock).toHaveBeenCalledWith('outer')
        expect(originalConsoleError).toHaveBeenCalledWith('nested')
    })

    test('plugin-log 가 rejected promise 를 돌려줘도 unhandledrejection 은 발생하지 않는다', async () => {
        logErrorSink = () => Promise.reject(new Error('ipc failed'))
        const unhandledRejectionListener = mock(() => {})
        process.on('unhandledRejection', unhandledRejectionListener)

        try {
            fakeConsole.error('boom')
            await new Promise((resolve) => setTimeout(resolve, 0))
        } finally {
            process.off('unhandledRejection', unhandledRejectionListener)
        }

        expect(unhandledRejectionListener).not.toHaveBeenCalled()
    })
})
