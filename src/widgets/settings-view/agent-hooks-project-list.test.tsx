import { describe, expect, test } from 'bun:test'
import type { AgentHooksStatus, CliInstallStatus } from '@shared/api/bindings'
import { QUERY_KEY } from '@shared/constants/query-key'
import { USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID } from '@entities/agent/agent.query'
import { createTestQueryClient, renderWithProviders, screen, within } from '@shared/testing/render'
import { AgentHooksProjectList } from '@widgets/settings-view/agent-hooks-project-list'

/**
 * Locks the one thing the settings rows may not decide for themselves (d-60 lens review B-1): which
 * agent needs the `taide` CLI. That answer lives in `AGENT_SPECS.delivery` on the Rust side and
 * arrives as `AgentHooksStatus.requiresTaideCli`, so these tests drive the gate purely by what the
 * seeded hook status says and never by the agent's name — a frontend table that re-stated the
 * delivery column would keep the warning pinned to `gemini` and fail the second test.
 *
 * No IPC is stubbed. Every `agent` query is seeded into the cache and disabled through
 * `setQueryDefaults`, which keeps `mock.module` (process-global and last-one-wins,
 * `docs/memory/test-conventions.md` §3) out of this file entirely. Labels are locale keys because
 * the test i18n instance carries no bundles.
 */

const CLI_MISSING_WARNING = 'settings.agentHooksCliMissing'

const GEMINI_LABEL = 'settings.agentHooksAgentGemini'
const CODEX_LABEL = 'settings.agentHooksAgentCodex'
const PI_LABEL = 'settings.agentHooksAgentPi'

const hooksStatus = (agentName: string, requiresTaideCli: boolean): AgentHooksStatus => ({
    agentName,
    scope: 'user',
    installed: false,
    requiresTaideCli,
})

const cliStatus = (installed: boolean): CliInstallStatus => ({ installed, targetPath: '/usr/local/bin/taide', editorEnvHint: '' })

type Fixture = { cliInstalled: boolean; requiresTaideCli: Record<string, boolean> }

/**
 * Agents left out of `requiresTaideCli` get no seeded status at all, which is how the
 * "server has not answered yet" state is expressed — their query stays disabled and empty.
 */
const renderList = ({ cliInstalled, requiresTaideCli }: Fixture) => {
    const queryClient = createTestQueryClient()
    queryClient.setQueryDefaults(QUERY_KEY.AGENT.ALL, { enabled: false })
    queryClient.setQueryData(QUERY_KEY.AGENT.CLI, cliStatus(cliInstalled))
    for (const [agentName, requires] of Object.entries(requiresTaideCli)) {
        queryClient.setQueryData(QUERY_KEY.AGENT.HOOKS(USER_LEVEL_AGENT_HOOKS_UNUSED_PROJECT_ID, agentName), hooksStatus(agentName, requires))
    }

    return renderWithProviders(<AgentHooksProjectList projects={[]} />, { queryClient })
}

/** A user-level row is the `label` wrapping its own switch; nothing else gives it a queryable handle. */
const userLevelRow = (labelKey: string) => {
    const row = screen.getByText(labelKey).closest('label')
    if (!row) throw new Error(`no user-level row for ${labelKey}`)
    return row
}

const hasCliWarning = (labelKey: string) => within(userLevelRow(labelKey)).queryByText(CLI_MISSING_WARNING) !== null

const isToggleDisabled = (labelKey: string) => within(userLevelRow(labelKey)).getByRole('switch').hasAttribute('disabled')

describe('AgentHooksProjectList', () => {
    test('CLI 미설치 경고와 토글 잠금은 훅 상태 응답의 requiresTaideCli 를 따른다', () => {
        renderList({ cliInstalled: false, requiresTaideCli: { codex: false, gemini: true, opencode: false, pi: false } })

        expect(screen.getAllByText(CLI_MISSING_WARNING)).toHaveLength(1)
        expect(hasCliWarning(GEMINI_LABEL)).toBe(true)
        expect(isToggleDisabled(GEMINI_LABEL)).toBe(true)
        expect(hasCliWarning(CODEX_LABEL)).toBe(false)
        expect(isToggleDisabled(CODEX_LABEL)).toBe(false)
    })

    test('서버가 값을 옮기면 경고도 그 행으로 옮겨간다', () => {
        renderList({ cliInstalled: false, requiresTaideCli: { codex: true, gemini: false, opencode: false, pi: false } })

        expect(screen.getAllByText(CLI_MISSING_WARNING)).toHaveLength(1)
        expect(hasCliWarning(CODEX_LABEL)).toBe(true)
        expect(isToggleDisabled(CODEX_LABEL)).toBe(true)
        expect(hasCliWarning(GEMINI_LABEL)).toBe(false)
        expect(isToggleDisabled(GEMINI_LABEL)).toBe(false)
    })

    test('CLI 가 설치돼 있으면 CLI 를 요구하는 행도 경고 없이 열린다', () => {
        renderList({ cliInstalled: true, requiresTaideCli: { codex: false, gemini: true, opencode: false, pi: false } })

        expect(screen.queryByText(CLI_MISSING_WARNING)).toBeNull()
        expect(isToggleDisabled(GEMINI_LABEL)).toBe(false)
    })

    test('훅 상태를 아직 모르는 행은 경고도 띄우지 않고 토글도 열지 않는다', () => {
        renderList({ cliInstalled: false, requiresTaideCli: { codex: false, gemini: true, opencode: false } })

        expect(hasCliWarning(PI_LABEL)).toBe(false)
        expect(isToggleDisabled(PI_LABEL)).toBe(true)
    })
})
