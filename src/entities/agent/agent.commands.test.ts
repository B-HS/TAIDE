import { beforeEach, describe, expect, spyOn, test } from 'bun:test'
import { toast } from 'sonner'
import { AGENT_CLI_COMMANDS } from '@entities/agent/agent.commands'
import { commands } from '@shared/api/bindings'
import type { CliInstallStatus } from '@shared/api/bindings'
import type { CommandContext } from '@shared/lib/command-registry'

const INSTALLED_STATUS: CliInstallStatus = {
    installed: true,
    resolvedPath: '/Applications/TAIDE.app/Contents/MacOS/taide-cli',
    dangling: false,
    targetPath: '/usr/local/bin/taide',
    editorEnvHint: 'export EDITOR="taide --wait"',
}

const MISSING_STATUS: CliInstallStatus = { ...INSTALLED_STATUS, installed: false, resolvedPath: null }

const DANGLING_STATUS: CliInstallStatus = { ...INSTALLED_STATUS, resolvedPath: null, dangling: true }

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

let currentStatus: CliInstallStatus = INSTALLED_STATUS
let installResult: CliInstallStatus = INSTALLED_STATUS
let statusFails = false
let installCallCount = 0
const successMessages: string[] = []
const errorMessages: string[] = []

const runConnectCommand = async () => {
    const command = AGENT_CLI_COMMANDS.find((entry) => entry.id === 'cli.connectExternalEditor')
    expect(command).toBeDefined()
    await command?.run(dummyContext)
}

describe('cli.connectExternalEditor 커맨드', () => {
    beforeEach(() => {
        currentStatus = INSTALLED_STATUS
        installResult = INSTALLED_STATUS
        statusFails = false
        installCallCount = 0
        successMessages.length = 0
        errorMessages.length = 0
        spyOn(commands, 'agentCliStatus').mockImplementation(async () => {
            if (statusFails) throw new Error('status failed')
            return { status: 'ok', data: currentStatus }
        })
        spyOn(commands, 'agentCliInstall').mockImplementation(async () => {
            installCallCount += 1
            return { status: 'ok', data: installResult }
        })
        spyOn(commands, 'agentCliUninstall').mockResolvedValue({ status: 'ok', data: MISSING_STATUS })
        spyOn(toast, 'success').mockImplementation((message) => {
            successMessages.push(String(message))
            return 'test'
        })
        spyOn(toast, 'error').mockImplementation((message) => {
            errorMessages.push(String(message))
            return 'test'
        })
    })

    test('CLI 가 이미 설치되어 있으면 다시 설치하지 않고 연결 안내만 띄운다', async () => {
        await runConnectCommand()
        expect(installCallCount).toBe(0)
        expect(successMessages).toEqual(['settings.cliExternalEditorConnected'])
    })

    test('CLI 가 없으면 설치한 뒤 연결 안내를 띄운다', async () => {
        currentStatus = MISSING_STATUS
        await runConnectCommand()
        expect(installCallCount).toBe(1)
        expect(successMessages).toEqual(['settings.cliExternalEditorConnected'])
    })

    test('심링크가 dangling 이면 재설치한다', async () => {
        currentStatus = DANGLING_STATUS
        await runConnectCommand()
        expect(installCallCount).toBe(1)
    })

    test('설치를 실행했는데도 심링크가 없으면 연결 안내 대신 실패 토스트를 띄운다', async () => {
        currentStatus = MISSING_STATUS
        installResult = MISSING_STATUS
        await runConnectCommand()
        expect(installCallCount).toBe(1)
        expect(successMessages).toEqual([])
        expect(errorMessages).toEqual(['settings.cliInstallFailed'])
    })

    test('설치 상태 조회가 실패하면 실패 토스트를 띄운다', async () => {
        statusFails = true
        await runConnectCommand()
        expect(installCallCount).toBe(0)
        expect(successMessages).toEqual([])
        expect(errorMessages).toEqual(['settings.cliInstallFailed'])
    })
})
