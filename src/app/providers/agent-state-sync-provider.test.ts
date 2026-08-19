import { describe, expect, test } from 'bun:test'
import { AgentStateSyncProvider } from '@app/providers/agent-state-sync-provider'

describe('AgentStateSyncProvider 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', () => {
        expect(typeof AgentStateSyncProvider).toBe('function')
    })
})
