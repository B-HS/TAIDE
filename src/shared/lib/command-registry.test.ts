import { afterEach, describe, expect, spyOn, test } from 'bun:test'
import { toast } from 'sonner'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    clearCommandRegistry,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    registerCommand,
    registerCommands,
    runCommandSafely,
    unregisterCommand,
} from '@shared/lib/command-registry'

const dummyContext: CommandContext = {
    activeProjectId: null,
    focusedShellSlotId: null,
    activeEditorActionIds: null,
    openSettingsTab: () => {},
    openSettingsFile: () => {},
    openTerminalTab: () => {},
    openWelcomeTab: () => {},
    reopenClosedTab: () => {},
    switchToFileSearchMode: () => {},
}

const buildCommand = (overrides: Partial<AppCommand> = {}): AppCommand => ({
    id: 'test.command',
    titleKey: 'test.title',
    run: () => {},
    ...overrides,
})

afterEach(() => {
    clearCommandRegistry()
})

describe('registerCommand / getRegisteredCommand', () => {
    test('등록한 커맨드를 id 로 조회할 수 있다', () => {
        registerCommand(buildCommand({ id: 'a' }))
        expect(getRegisteredCommand('a')?.id).toBe('a')
    })

    test('등록되지 않은 id 는 null 을 반환한다', () => {
        expect(getRegisteredCommand('missing')).toBeNull()
    })

    test('같은 id 로 다시 등록하면 이전 커맨드를 덮어쓴다', () => {
        registerCommand(buildCommand({ id: 'a', titleKey: 'first' }))
        registerCommand(buildCommand({ id: 'a', titleKey: 'second' }))
        expect(getRegisteredCommand('a')?.titleKey).toBe('second')
        expect(listRegisteredCommands()).toHaveLength(1)
    })
})

describe('registerCommands / unregisterCommand / clearCommandRegistry', () => {
    test('여러 커맨드를 한 번에 등록한다', () => {
        registerCommands([buildCommand({ id: 'a' }), buildCommand({ id: 'b' })])
        expect(
            listRegisteredCommands()
                .map((command) => command.id)
                .sort(),
        ).toEqual(['a', 'b'])
    })

    test('unregisterCommand 는 해당 id 만 제거한다', () => {
        registerCommands([buildCommand({ id: 'a' }), buildCommand({ id: 'b' })])
        unregisterCommand('a')
        expect(listRegisteredCommands().map((command) => command.id)).toEqual(['b'])
    })

    test('clearCommandRegistry 는 등록된 전체 커맨드를 비운다', () => {
        registerCommands([buildCommand({ id: 'a' }), buildCommand({ id: 'b' })])
        clearCommandRegistry()
        expect(listRegisteredCommands()).toEqual([])
    })
})

describe('isCommandRunnable', () => {
    test('isEnabled 가 없으면 실행 가능으로 취급한다', () => {
        expect(isCommandRunnable(buildCommand(), dummyContext)).toBe(true)
    })

    test('isEnabled 가 false 를 반환하면 실행 불가로 취급한다', () => {
        expect(isCommandRunnable(buildCommand({ isEnabled: () => false }), dummyContext)).toBe(false)
    })

    test('isEnabled 는 컨텍스트 값을 근거로 판단할 수 있다', () => {
        const command = buildCommand({ isEnabled: (context) => context.activeProjectId !== null })
        expect(isCommandRunnable(command, dummyContext)).toBe(false)
        expect(isCommandRunnable(command, { ...dummyContext, activeProjectId: 'p1' })).toBe(true)
    })
})

const REJECTION_FLUSH_TICKS = 5

/**
 * Drains the microtask queue far enough for an async `run` to settle and for the helper's own
 * rejection handler to run after it — an `async` function that *returns* a rejected promise (the
 * shape `terminal.copyImeDebug` has) adopts it over several ticks, so a single `await` is short.
 */
const flushRejectionHandling = async () => {
    for (let tick = 0; tick < REJECTION_FLUSH_TICKS; tick += 1) await Promise.resolve()
}

/**
 * The two palette dispatch sites hand `run` straight to this helper, so both shapes a command can
 * fail in have to end at the same reporter — a synchronous throw never becomes a promise, and a
 * rejected one never reaches a `catch` block.
 */
describe('runCommandSafely', () => {
    test('동기 run 이 throw 해도 전파하지 않고 실패를 토스트로 보고한다', () => {
        const toastError = spyOn(toast, 'error')

        runCommandSafely(
            buildCommand({
                run: () => {
                    throw new Error('sync boom')
                },
            }),
            dummyContext,
        )

        expect(toastError).toHaveBeenCalledWith('sync boom')
    })

    test('async run 이 reject 하면 unhandled rejection 으로 흘리지 않고 실패를 토스트로 보고한다', async () => {
        const toastError = spyOn(toast, 'error')

        runCommandSafely(buildCommand({ run: async () => Promise.reject(new Error('async boom')) }), dummyContext)
        await flushRejectionHandling()

        expect(toastError).toHaveBeenCalledWith('async boom')
    })

    test('정상적으로 끝난 커맨드는 아무것도 보고하지 않는다', async () => {
        const toastError = spyOn(toast, 'error')
        const calls: string[] = []

        runCommandSafely(buildCommand({ run: () => void calls.push('sync') }), dummyContext)
        runCommandSafely(
            buildCommand({
                run: async () => {
                    calls.push('async')
                },
            }),
            dummyContext,
        )
        await flushRejectionHandling()

        expect(calls).toEqual(['sync', 'async'])
        expect(toastError).not.toHaveBeenCalled()
    })
})
