import { describe, expect, test } from 'bun:test'
import type { AgentCompletionCandidate, AgentWorkingSinceMap } from '@shared/lib/native-notification-gate'
import {
    evaluateAgentCompletions,
    shouldForwardNativeNotification,
    shouldNotifyTaskCompletion,
    truncateNotificationText,
} from '@shared/lib/native-notification-gate'

const MIN_WORKING_MS = 10_000

const agent = (sessionId: string, activity: AgentCompletionCandidate['activity'], name = 'claude'): AgentCompletionCandidate => ({
    sessionId,
    name,
    activity,
})

const evaluate = (workingSince: AgentWorkingSinceMap, agents: AgentCompletionCandidate[], nowMs: number) =>
    evaluateAgentCompletions({ workingSince, agents, nowMs, minWorkingMs: MIN_WORKING_MS })

describe('shouldForwardNativeNotification', () => {
    test('메인 창의 데스크톱 런타임만 전달한다', () => {
        expect(shouldForwardNativeNotification({ windowKind: 'main', isRemoteMirror: false })).toBe(true)
    })

    test('보조 창은 전달하지 않는다(같은 이벤트를 창마다 듣기 때문)', () => {
        expect(shouldForwardNativeNotification({ windowKind: 'auxiliary', isRemoteMirror: false })).toBe(false)
    })

    test('원격 미러는 메인 라벨이어도 전달하지 않는다', () => {
        expect(shouldForwardNativeNotification({ windowKind: 'main', isRemoteMirror: true })).toBe(false)
    })
})

describe('evaluateAgentCompletions', () => {
    test('working 을 처음 보면 시작 시각을 기록하고 알리지 않는다', () => {
        const result = evaluate({}, [agent('a', 'working')], 1_000)
        expect(result.workingSince).toEqual({ a: 1_000 })
        expect(result.completed).toEqual([])
    })

    test('working 이 이어지면 최초 시작 시각을 유지한다', () => {
        const result = evaluate({ a: 1_000 }, [agent('a', 'working')], 9_000)
        expect(result.workingSince).toEqual({ a: 1_000 })
        expect(result.completed).toEqual([])
    })

    test('임계 이상 working 후 idle 로 바뀌면 완료로 본다', () => {
        const result = evaluate({ a: 1_000 }, [agent('a', 'idle')], 1_000 + MIN_WORKING_MS)
        expect(result.completed).toEqual([{ sessionId: 'a', name: 'claude', workedForMs: MIN_WORKING_MS }])
        expect(result.workingSince).toEqual({})
    })

    test('awaitingInput 도 완료로 본다', () => {
        const result = evaluate({ a: 0 }, [agent('a', 'awaitingInput')], MIN_WORKING_MS)
        expect(result.completed.map((entry) => entry.sessionId)).toEqual(['a'])
    })

    test('임계 미만이면 알리지 않고 기록만 지운다', () => {
        const result = evaluate({ a: 1_000 }, [agent('a', 'idle')], 1_000 + MIN_WORKING_MS - 1)
        expect(result.completed).toEqual([])
        expect(result.workingSince).toEqual({})
    })

    test('working 을 본 적 없는 에이전트의 idle 은 완료가 아니다', () => {
        const result = evaluate({}, [agent('a', 'idle')], 100_000)
        expect(result.completed).toEqual([])
    })

    test('unknown 으로 빠지면 완료가 아니다', () => {
        const result = evaluate({ a: 0 }, [agent('a', 'unknown')], 100_000)
        expect(result.completed).toEqual([])
        expect(result.workingSince).toEqual({})
    })

    test('목록에서 사라진 에이전트는 알림 없이 기록에서 빠진다', () => {
        const result = evaluate({ a: 0, b: 0 }, [agent('b', 'working')], 100_000)
        expect(result.completed).toEqual([])
        expect(result.workingSince).toEqual({ b: 0 })
    })

    test('여러 에이전트가 동시에 끝나면 모두 보고한다', () => {
        const result = evaluate({ a: 0, b: 0 }, [agent('a', 'idle', 'claude'), agent('b', 'awaitingInput', 'codex')], MIN_WORKING_MS)
        expect(result.completed.map((entry) => entry.name)).toEqual(['claude', 'codex'])
    })

    test('세 스냅샷을 거쳐도 최초 working 시각으로 작업 시간을 잰다', () => {
        const first = evaluate({}, [agent('a', 'working')], 1_000)
        const second = evaluate(first.workingSince, [agent('a', 'working')], 5_000)
        const third = evaluate(second.workingSince, [agent('a', 'idle')], 1_000 + MIN_WORKING_MS)

        expect(third.completed).toEqual([{ sessionId: 'a', name: 'claude', workedForMs: MIN_WORKING_MS }])
    })

    test('한 에이전트가 끝나도 아직 일하는 다른 에이전트의 시작 시각은 유지된다', () => {
        const result = evaluate({ a: 0, b: 500 }, [agent('a', 'idle'), agent('b', 'working')], MIN_WORKING_MS)

        expect(result.completed.map((entry) => entry.sessionId)).toEqual(['a'])
        expect(result.workingSince).toEqual({ b: 500 })
    })

    test('완료 후 다시 working 이 되면 새 시작 시각으로 다시 잰다', () => {
        const finished = evaluate({ a: 0 }, [agent('a', 'idle')], MIN_WORKING_MS)
        const restarted = evaluate(finished.workingSince, [agent('a', 'working')], 50_000)

        expect(restarted.workingSince).toEqual({ a: 50_000 })
        expect(evaluate(restarted.workingSince, [agent('a', 'idle')], 50_000 + MIN_WORKING_MS - 1).completed).toEqual([])
    })

    test('빈 목록은 기록을 전부 비우고 아무것도 알리지 않는다', () => {
        expect(evaluate({ a: 0 }, [], 100_000)).toEqual({ workingSince: {}, completed: [] })
    })

    test('완료 보고에는 에이전트 이름이 그대로 실린다', () => {
        const result = evaluate({ a: 0 }, [agent('a', 'idle', 'gemini')], MIN_WORKING_MS)
        expect(result.completed[0]?.name).toBe('gemini')
    })

    test('입력 맵을 변형하지 않는다', () => {
        const workingSince = { a: 0 }
        evaluate(workingSince, [agent('a', 'idle')], MIN_WORKING_MS)
        expect(workingSince).toEqual({ a: 0 })
    })
})

describe('shouldNotifyTaskCompletion', () => {
    test('임계 이상이면 알린다', () => {
        expect(shouldNotifyTaskCompletion({ durationMs: MIN_WORKING_MS, minDurationMs: MIN_WORKING_MS })).toBe(true)
    })

    test('임계 미만이면 알리지 않는다', () => {
        expect(shouldNotifyTaskCompletion({ durationMs: MIN_WORKING_MS - 1, minDurationMs: MIN_WORKING_MS })).toBe(false)
    })

    test('즉시 끝난 명령(0ms)은 알리지 않는다', () => {
        expect(shouldNotifyTaskCompletion({ durationMs: 0, minDurationMs: MIN_WORKING_MS })).toBe(false)
    })

    test('임계가 0 이면 모든 명령을 알린다', () => {
        expect(shouldNotifyTaskCompletion({ durationMs: 0, minDurationMs: 0 })).toBe(true)
    })
})

describe('truncateNotificationText', () => {
    test('상한 이하 문자열은 그대로 둔다', () => {
        expect(truncateNotificationText('build ok', 10)).toBe('build ok')
    })

    test('상한을 넘으면 말줄임표까지 포함해 상한 길이로 자른다', () => {
        const truncated = truncateNotificationText('abcdefghij', 5)
        expect(truncated).toBe('abcd…')
        expect([...truncated]).toHaveLength(5)
    })

    test('서로게이트 쌍을 반으로 쪼개지 않는다', () => {
        expect(truncateNotificationText('🚀🚀🚀', 2)).toBe('🚀…')
    })

    test('코드포인트 수가 정확히 상한이면 자르지 않는다 — 이모지는 UTF-16 길이가 상한을 넘어도 유지', () => {
        const exact = '🚀🚀🚀🚀🚀'
        expect(exact.length).toBeGreaterThan(5)
        expect(truncateNotificationText(exact, 5)).toBe(exact)
    })

    test('상한이 1 이면 말줄임표만 남는다', () => {
        expect(truncateNotificationText('abc', 1)).toBe('…')
    })

    test('빈 문자열은 그대로 둔다', () => {
        expect(truncateNotificationText('', 5)).toBe('')
    })
})
