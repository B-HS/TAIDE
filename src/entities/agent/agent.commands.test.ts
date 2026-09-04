import { beforeEach, describe, expect, mock, test } from 'bun:test'
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

/**
 * `agent.ipc` reaches `@shared/api/bindings`, i.e. the Tauri `invoke` bridge that only exists inside
 * a webview — stubbing the entity's own IPC module keeps the command's decision logic (does it
 * install, and what does it report?) testable without one.
 */
mock.module('@entities/agent/agent.ipc', () => ({
    getCliInstallStatus: async () => {
        if (statusFails) throw new Error('status failed')
        return currentStatus
    },
    installCliCommand: async () => {
        installCallCount += 1
        return installResult
    },
    uninstallCliCommand: async () => MISSING_STATUS,
}))

mock.module('sonner', () => ({
    toast: {
        success: (message: string) => successMessages.push(message),
        error: (message: string) => errorMessages.push(message),
    },
}))

const { AGENT_CLI_COMMANDS } = await import('@entities/agent/agent.commands')

/**
 * `AGENT_CLI_COMMANDS` is `IS_MAC`-gated and `bun test` sees Bun's own `navigator.platform`, so
 * these cases exercise the macOS registration — the only platform where the backend accepts the CLI
 * shell-command install at all, and the only one the release pipeline builds for.
 */
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
