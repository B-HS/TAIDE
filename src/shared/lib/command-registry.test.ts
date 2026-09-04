import { afterEach, describe, expect, test } from 'bun:test'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    clearCommandRegistry,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    registerCommand,
    registerCommands,
    unregisterCommand,
} from '@shared/lib/command-registry'

const dummyContext: CommandContext = {
    activeProjectId: null,
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
