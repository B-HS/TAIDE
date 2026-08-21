import { describe, expect, test } from 'bun:test'
import type { AppCommand, CommandContext } from '@shared/lib/command-registry'
import { isCommandRunnable } from '@shared/lib/command-registry'
import { DEFAULT_COMMANDS } from '@shared/lib/command-catalog'
import { setImeDebugEnabled } from '@shared/lib/ime-debug'

const dummyContext: CommandContext = {
    activeProjectId: null,
    activeEditorActionIds: null,
    openSettingsTab: () => {},
    openSettingsFile: () => {},
    openTerminalTab: () => {},
    reopenClosedTab: () => {},
    switchToFileSearchMode: () => {},
}

describe('DEFAULT_COMMANDS', () => {
    test('id 가 서로 중복되지 않는다', () => {
        const ids = DEFAULT_COMMANDS.map((command) => command.id)
        expect(new Set(ids).size).toBe(ids.length)
    })

    test('실행 구현이 있는 기본 커맨드는 항상 실행 가능하다', () => {
        const runnableIds = [
            'window.reload',
            'settings.open',
            'app.openSettingsFile',
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
            'tab.moveToNewWindow',
        ]
        for (const id of runnableIds) {
            const command = DEFAULT_COMMANDS.find((entry) => entry.id === id)
            expect(command).toBeDefined()
            expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(true)
        }
    })

    test('실행 구현이 없는 keymap 미러 커맨드(탭 닫기·파일 내 찾기)는 항상 비활성이다', () => {
        for (const id of ['tab.close', 'editor.find']) {
            const command = DEFAULT_COMMANDS.find((entry) => entry.id === id)
            expect(command).toBeDefined()
            expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(false)
        }
    })

    /**
     * `tab.moveToMainWindow` gates on `getWindowContext()` (module-scoped, not `CommandContext`) —
     * `bun:test` has no `window` global, so `getWindowContext()` always resolves to the main window
     * here and only the disabled branch is reachable from this test file. The enabled branch is
     * covered by `window-context.test.ts`'s `readWindowContext` tests instead.
     */
    test('terminal.copyImeDebug 는 진단 플래그가 켜졌을 때만 실행 가능하다', () => {
        const command = DEFAULT_COMMANDS.find((entry) => entry.id === 'terminal.copyImeDebug')
        expect(command).toBeDefined()
        expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(false)

        setImeDebugEnabled(true)
        expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(true)
        setImeDebugEnabled(false)
    })

    test('tab.moveToMainWindow 은 window-context 기반으로 활성 여부를 판단한다(테스트 환경엔 window 전역이 없어 비활성 경로만 검증)', () => {
        const command = DEFAULT_COMMANDS.find((entry) => entry.id === 'tab.moveToMainWindow')
        expect(command).toBeDefined()
        expect(isCommandRunnable(command as AppCommand, dummyContext)).toBe(false)
    })
})
