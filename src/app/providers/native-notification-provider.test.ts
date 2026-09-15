import { describe, expect, test } from 'bun:test'
import { applyLocaleMessages, i18next } from '@shared/i18n/i18n'
import type { AgentCompletionEvaluation } from '@shared/lib/native-notification-gate'

applyLocaleMessages(i18next.language, {
    'common.durationHoursMinutes': '{{hours}}h {{minutes}}m',
    'common.durationMinutesSeconds': '{{minutes}}m {{seconds}}s',
    'common.durationSeconds': '{{seconds}}s',
    'notification.agentAwaitingInputBody': '{{agent}} · Waiting for input (permission request or question){{tab}}',
    'notification.agentPermissionBody': '{{agent}} · Waiting for permission{{tab}}',
    'notification.agentQuestionBody': '{{agent}} · Waiting for an answer{{tab}}',
    'notification.agentFinishedBody': '{{agent}} · Finished · {{duration}}{{tab}}',
    'notification.exitCode': 'Exit code {{code}}',
})

const MINUTE_MS = 60_000

type AgentCompletion = AgentCompletionEvaluation['completed'][number]

const buildCompletion = (overrides: Partial<AgentCompletion> = {}): AgentCompletion => ({
    sessionId: 's1',
    name: 'claude',
    workedForMs: 3 * MINUTE_MS + 12_000,
    activity: 'idle',
    blockedReason: null,
    ...overrides,
})

const importProvider = () => import('@app/providers/native-notification-provider')

describe('NativeNotificationProvider 모듈 로드', () => {
    test('컴포넌트 함수로 export 된다', async () => {
        expect(typeof (await importProvider()).NativeNotificationProvider).toBe('function')
    })
})

describe('buildAgentCompletionBody', () => {
    test('idle 은 에이전트 이름·작업 완료·경과 시간을 싣는다', async () => {
        const { buildAgentCompletionBody } = await importProvider()
        expect(buildAgentCompletionBody(buildCompletion(), null)).toBe('claude · Finished · 3m 12s')
    })

    test('터미널 탭 제목이 있으면 꼬리로 붙는다', async () => {
        const { buildAgentCompletionBody } = await importProvider()
        expect(buildAgentCompletionBody(buildCompletion(), 'agent:claude')).toBe('claude · Finished · 3m 12s · agent:claude')
    })

    test('사유를 모르는 awaitingInput 은 둘 다 아우르는 문구를 쓰고 경과 시간을 싣지 않는다', async () => {
        const { buildAgentCompletionBody } = await importProvider()
        expect(buildAgentCompletionBody(buildCompletion({ activity: 'awaitingInput' }), 'agent:claude')).toBe(
            'claude · Waiting for input (permission request or question) · agent:claude',
        )
    })

    test('permission · question · dialog 사유별로 문구가 갈린다(dialog 는 일반 문구로 폴백)', async () => {
        const { buildAgentCompletionBody } = await importProvider()
        const body = (blockedReason: AgentCompletion['blockedReason']) =>
            buildAgentCompletionBody(buildCompletion({ activity: 'awaitingInput', blockedReason }), null)
        expect(body('permission')).toBe('claude · Waiting for permission')
        expect(body('question')).toBe('claude · Waiting for an answer')
        expect(body('dialog')).toBe('claude · Waiting for input (permission request or question)')
    })

    test('idle 은 사유가 실려 있어도 완료 문구를 쓴다', async () => {
        const { buildAgentCompletionBody } = await importProvider()
        expect(buildAgentCompletionBody(buildCompletion({ blockedReason: 'permission' }), null)).toBe('claude · Finished · 3m 12s')
    })
})

describe('buildTaskCompletionBody', () => {
    test('탭 제목·종료 코드·경과 시간을 잇는다', async () => {
        const { buildTaskCompletionBody } = await importProvider()
        expect(buildTaskCompletionBody({ target: 'bash', exitCode: 1, durationMs: 45_000 })).toBe('bash · Exit code 1 · 45s')
    })

    test('성공한 명령도 종료 코드 0 을 보여준다 (제목이 프로젝트명이라 본문만이 결과를 말한다)', async () => {
        const { buildTaskCompletionBody } = await importProvider()
        expect(buildTaskCompletionBody({ target: 'bash', exitCode: 0, durationMs: 45_000 })).toBe('bash · Exit code 0 · 45s')
    })

    test('셸이 종료 코드를 보고하지 않았으면 그 조각을 만들지 않는다', async () => {
        const { buildTaskCompletionBody } = await importProvider()
        expect(buildTaskCompletionBody({ target: '/tmp/work', exitCode: null, durationMs: 45_000 })).toBe('/tmp/work · 45s')
    })
})
