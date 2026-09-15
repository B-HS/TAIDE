import { describe, expect, test } from 'bun:test'
import type { AgentActivity } from '@shared/api/bindings'
import { agentStatusLabelKey } from '@shared/lib/agent-status-text'

describe('agentStatusLabelKey', () => {
    test('차단이 아닌 활동은 사유를 무시하고 활동 키를 쓴다', () => {
        const activities: AgentActivity[] = ['working', 'idle', 'unknown']
        expect(activities.map((activity) => agentStatusLabelKey(activity, 'permission'))).toEqual([
            'agent.status.working',
            'agent.status.idle',
            'agent.status.unknown',
        ])
    })

    test('입력 대기는 사유별 키로 갈린다', () => {
        expect(agentStatusLabelKey('awaitingInput', 'permission')).toBe('agent.blocked.permission')
        expect(agentStatusLabelKey('awaitingInput', 'question')).toBe('agent.blocked.question')
        expect(agentStatusLabelKey('awaitingInput', 'dialog')).toBe('agent.blocked.dialog')
    })

    test('사유 없는 입력 대기(히스테리시스·HTTP override)는 일반 상태 키로 돌아간다', () => {
        expect(agentStatusLabelKey('awaitingInput', null)).toBe('agent.status.awaitingInput')
        expect(agentStatusLabelKey('awaitingInput', undefined)).toBe('agent.status.awaitingInput')
    })
})
