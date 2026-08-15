import { afterEach, describe, expect, test } from 'bun:test'
import type { languages } from 'monaco-editor'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import {
    DEFAULT_COMMANDS,
    buildCommandModeQuery,
    clearCommandRegistry,
    flattenDocumentSymbols,
    getRegisteredCommand,
    isCommandRunnable,
    listRegisteredCommands,
    parseLineModeTarget,
    parsePaletteQuery,
    registerCommand,
    registerCommands,
    unregisterCommand,
} from '@shared/lib/command-registry'

const dummyContext: CommandContext = {
    activeProjectId: null,
    activeEditorActionIds: null,
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

    test('"@" 로 시작하면 symbol 모드다(문서 심볼)', () => {
        expect(parsePaletteQuery('@handleSave')).toEqual({ mode: 'symbol', searchTerm: 'handleSave' })
    })

    test('"@" 단독 입력은 검색어가 빈 symbol 모드다', () => {
        expect(parsePaletteQuery('@')).toEqual({ mode: 'symbol', searchTerm: '' })
    })

    test('":" 로 시작하면 line 모드다(줄 이동)', () => {
        expect(parsePaletteQuery(':123')).toEqual({ mode: 'line', searchTerm: '123' })
    })

    test('":" 뒤 "줄:열" 표기도 검색어로 그대로 보존한다', () => {
        expect(parsePaletteQuery(':123:45')).toEqual({ mode: 'line', searchTerm: '123:45' })
    })

    test('"#" 로 시작하면 workspaceSymbol 모드다(워크스페이스 심볼)', () => {
        expect(parsePaletteQuery('#handleSave')).toEqual({ mode: 'workspaceSymbol', searchTerm: 'handleSave' })
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

describe('parseLineModeTarget', () => {
    test('숫자만 있으면 1열로 취급한다', () => {
        expect(parseLineModeTarget('123')).toEqual({ line: 123, column: 1 })
    })

    test('"줄:열" 표기를 파싱한다', () => {
        expect(parseLineModeTarget('123:45')).toEqual({ line: 123, column: 45 })
    })

    test('앞뒤 공백은 무시한다', () => {
        expect(parseLineModeTarget('  42  ')).toEqual({ line: 42, column: 1 })
    })

    test('빈 문자열은 null 이다', () => {
        expect(parseLineModeTarget('')).toBeNull()
    })

    test('숫자가 아닌 입력은 null 이다', () => {
        expect(parseLineModeTarget('abc')).toBeNull()
    })

    test('0 이하의 줄/열은 null 이다', () => {
        expect(parseLineModeTarget('0')).toBeNull()
        expect(parseLineModeTarget('1:0')).toBeNull()
    })

    test('콜론이 2개 이상이면 형식이 맞지 않아 null 이다', () => {
        expect(parseLineModeTarget('1:2:3')).toBeNull()
    })
})

describe('flattenDocumentSymbols', () => {
    const buildSymbol = (overrides: Partial<languages.DocumentSymbol> = {}): languages.DocumentSymbol => ({
        name: 'symbol',
        detail: '',
        kind: 12,
        tags: [],
        range: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 },
        selectionRange: { startLineNumber: 1, startColumn: 1, endLineNumber: 1, endColumn: 1 },
        ...overrides,
    })

    test('빈 배열은 빈 배열을 반환한다', () => {
        expect(flattenDocumentSymbols([])).toEqual([])
    })

    test('최상위 심볼은 containerLabel 이 빈 문자열이다', () => {
        const result = flattenDocumentSymbols([buildSymbol({ name: 'handleSave' })])
        expect(result).toEqual([{ name: 'handleSave', detail: '', kind: 12, containerLabel: '', selectionRange: buildSymbol().selectionRange }])
    })

    test('children 을 재귀적으로 평탄화하며 조상 이름을 "부모 > 자식" 형태의 containerLabel 로 쌓는다', () => {
        const result = flattenDocumentSymbols([
            buildSymbol({
                name: 'MyClass',
                children: [buildSymbol({ name: 'method', children: [buildSymbol({ name: 'inner' })] })],
            }),
        ])

        expect(result.map((symbol) => [symbol.name, symbol.containerLabel])).toEqual([
            ['MyClass', ''],
            ['method', 'MyClass'],
            ['inner', 'MyClass > method'],
        ])
    })

    test('형제 심볼은 서로 다른 depth 로 섞이지 않고 순서대로 나열된다', () => {
        const result = flattenDocumentSymbols([buildSymbol({ name: 'a', children: [buildSymbol({ name: 'a1' })] }), buildSymbol({ name: 'b' })])

        expect(result.map((symbol) => symbol.name)).toEqual(['a', 'a1', 'b'])
    })
})
