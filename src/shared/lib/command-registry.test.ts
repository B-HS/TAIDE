import { afterEach, describe, expect, test } from 'bun:test'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    DEFAULT_COMMANDS,
    buildCommandModeQuery,
    clearCommandRegistry,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    parsePaletteQuery,
    registerCommand,
    registerCommands,
    unregisterCommand,
} from '@shared/lib/command-registry'

const dummyContext: CommandContext = {
    activeProjectId: null,
    openSettingsTab: () => {},
    openTerminalTab: () => {},
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

describe('parsePaletteQuery', () => {
    test('">" 로 시작하지 않으면 파일 모드로 판단하고 입력 전체를 검색어로 본다', () => {
        expect(parsePaletteQuery('foo.ts')).toEqual({ mode: 'files', searchTerm: 'foo.ts' })
    })

    test('빈 입력은 파일 모드다', () => {
        expect(parsePaletteQuery('')).toEqual({ mode: 'files', searchTerm: '' })
    })

    test('">" 로 시작하면 커맨드 모드로 판단하고 접두사를 제거한 나머지를 검색어로 본다', () => {
        expect(parsePaletteQuery('>reload')).toEqual({ mode: 'commands', searchTerm: 'reload' })
    })

    test('">" 단독 입력은 검색어가 빈 커맨드 모드다', () => {
        expect(parsePaletteQuery('>')).toEqual({ mode: 'commands', searchTerm: '' })
    })

    test('">" 뒤 공백은 검색어에서 제거된다', () => {
        expect(parsePaletteQuery('>   reload window')).toEqual({ mode: 'commands', searchTerm: 'reload window' })
    })
})

describe('buildCommandModeQuery', () => {
    test('검색어 없이 호출하면 ">" 만 반환한다', () => {
        expect(buildCommandModeQuery()).toBe('>')
    })

    test('검색어를 붙이면 ">검색어" 형태로 반환한다', () => {
        expect(buildCommandModeQuery('reload')).toBe('>reload')
    })
})

describe('DEFAULT_COMMANDS', () => {
    test('id 가 서로 중복되지 않는다', () => {
        const ids = DEFAULT_COMMANDS.map((command) => command.id)
        expect(new Set(ids).size).toBe(ids.length)
    })

    test('실행 구현이 있는 기본 커맨드는 항상 실행 가능하다', () => {
        const runnableIds = [
            'window.reload',
            'settings.open',
            'terminal.new',
            'tab.reopenClosed',
            'file.quickOpen',
            'search.find',
            'search.replace',
            'view.toggleSidebar',
            'view.explorer',
            'view.git',
            'editor.split',
            'tab.cycleNext',
            'tab.cyclePrev',
            'editor.save',
            'view.toggleTerminal',
        ]
        for (const id of runnableIds) {
            const command = DEFAULT_COMMANDS.find((entry) => entry.id === id)
            expect(command).toBeDefined()
            expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(true)
        }
    })

    test('실행 구현이 없는 keymap 미러 커맨드(탭 닫기·파일 내 찾기)는 항상 비활성이다', () => {
        const disabledIds = DEFAULT_COMMANDS.filter((command) => command.isEnabled).map((command) => command.id)
        expect(disabledIds).toEqual(['tab.close', 'editor.find'])
        for (const command of DEFAULT_COMMANDS) {
            if (!command.isEnabled) continue
            expect(isCommandRunnable(command, dummyContext)).toBe(false)
        }
    })
})
