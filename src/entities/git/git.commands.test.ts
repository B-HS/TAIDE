import { describe, expect, test } from 'bun:test'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { KEYMAP_CATEGORY, isCommandRunnable } from '@shared/lib/command-registry'
import { GIT_COMMANDS } from '@entities/git/git.commands'

const dummyContext: CommandContext = {
    activeProjectId: null,
    activeEditorActionIds: null,
    openSettingsTab: () => {},
    openTerminalTab: () => {},
    reopenClosedTab: () => {},
    switchToFileSearchMode: () => {},
}

describe('GIT_COMMANDS', () => {
    test('id 가 서로 중복되지 않는다', () => {
        const ids = GIT_COMMANDS.map((command) => command.id)
        expect(new Set(ids).size).toBe(ids.length)
    })

    test('모두 GIT 카테고리에 속한다', () => {
        for (const command of GIT_COMMANDS) {
            expect(command.categoryKey).toBe(KEYMAP_CATEGORY.GIT)
        }
    })

    test('활성 프로젝트가 없으면 revert·태그 커맨드는 비활성이다', () => {
        const revert = GIT_COMMANDS.find((command) => command.id === 'git.revertHead')
        const createTag = GIT_COMMANDS.find((command) => command.id === 'git.createTagOnHead')
        expect(revert).toBeDefined()
        expect(createTag).toBeDefined()
        expect(isCommandRunnable(revert as AppCommand, dummyContext)).toBe(false)
        expect(isCommandRunnable(createTag as AppCommand, dummyContext)).toBe(false)
    })

    test('활성 프로젝트가 있으면 revert·태그 커맨드는 활성이다', () => {
        const context: CommandContext = { ...dummyContext, activeProjectId: 'p1' }
        const revert = GIT_COMMANDS.find((command) => command.id === 'git.revertHead')
        const createTag = GIT_COMMANDS.find((command) => command.id === 'git.createTagOnHead')
        expect(isCommandRunnable(revert as AppCommand, context)).toBe(true)
        expect(isCommandRunnable(createTag as AppCommand, context)).toBe(true)
    })

    test('blame 토글 커맨드는 활성 프로젝트 여부와 무관하게 실행 가능하다', () => {
        const toggleBlame = GIT_COMMANDS.find((command) => command.id === 'git.toggleBlame')
        expect(toggleBlame).toBeDefined()
        expect(isCommandRunnable(toggleBlame as AppCommand, dummyContext)).toBe(true)
    })

    test('파일 히스토리 커맨드는 활성 프로젝트 여부와 무관하게 실행 가능하다', () => {
        const openFileHistory = GIT_COMMANDS.find((command) => command.id === 'git.openFileHistory')
        expect(openFileHistory).toBeDefined()
        expect(isCommandRunnable(openFileHistory as AppCommand, dummyContext)).toBe(true)
    })

    test('활성 프로젝트가 없을 때 revertHead 를 실행해도 IPC 를 호출하지 않고 반환한다', async () => {
        const revert = GIT_COMMANDS.find((command) => command.id === 'git.revertHead')
        expect(revert).toBeDefined()
        await expect(Promise.resolve(revert?.run(dummyContext))).resolves.toBeUndefined()
    })
})
