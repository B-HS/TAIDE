import { describe, expect, test } from 'bun:test'
import { HIGH_WATER_BYTES, LOW_WATER_BYTES } from '@shared/constants/terminal'
import { INITIAL_FLOW_CONTROL_STATE, evaluateFlowControl, shouldTogglePause } from '@widgets/terminal-pane/terminal-flow-control'

describe('evaluateFlowControl', () => {
    test('HIGH_WATER 이하이면 paused 를 유지한다', () => {
        const next = evaluateFlowControl(INITIAL_FLOW_CONTROL_STATE, HIGH_WATER_BYTES)
        expect(next).toEqual({ pendingBytes: HIGH_WATER_BYTES, paused: false })
    })

    test('HIGH_WATER 를 초과하면 paused 로 전환한다', () => {
        const next = evaluateFlowControl(INITIAL_FLOW_CONTROL_STATE, HIGH_WATER_BYTES + 1)
        expect(next).toEqual({ pendingBytes: HIGH_WATER_BYTES + 1, paused: true })
    })

    test('paused 상태에서 LOW_WATER 이상이면 paused 를 유지한다', () => {
        const pausedState = { pendingBytes: HIGH_WATER_BYTES + 1, paused: true }
        const next = evaluateFlowControl(pausedState, LOW_WATER_BYTES)
        expect(next).toEqual({ pendingBytes: LOW_WATER_BYTES, paused: true })
    })

    test('paused 상태에서 LOW_WATER 미만으로 떨어지면 재개한다', () => {
        const pausedState = { pendingBytes: HIGH_WATER_BYTES + 1, paused: true }
        const next = evaluateFlowControl(pausedState, LOW_WATER_BYTES - 1)
        expect(next).toEqual({ pendingBytes: LOW_WATER_BYTES - 1, paused: false })
    })

    test('paused 가 아니고 두 임계값 사이면 상태를 유지한다', () => {
        const state = { pendingBytes: 0, paused: false }
        const next = evaluateFlowControl(state, LOW_WATER_BYTES)
        expect(next).toEqual({ pendingBytes: LOW_WATER_BYTES, paused: false })
    })

    test('HIGH_WATER 경계값은 초과로 취급하지 않는다', () => {
        const next = evaluateFlowControl(INITIAL_FLOW_CONTROL_STATE, HIGH_WATER_BYTES)
        expect(next.paused).toBe(false)
    })

    test('LOW_WATER 경계값은 재개로 취급하지 않는다', () => {
        const pausedState = { pendingBytes: HIGH_WATER_BYTES + 1, paused: true }
        const next = evaluateFlowControl(pausedState, LOW_WATER_BYTES)
        expect(next.paused).toBe(true)
    })
})

describe('shouldTogglePause', () => {
    test('paused 값이 바뀌면 true 를 반환한다', () => {
        expect(shouldTogglePause({ pendingBytes: 0, paused: false }, { pendingBytes: 1, paused: true })).toBe(true)
    })

    test('paused 값이 같으면 false 를 반환한다', () => {
        expect(shouldTogglePause({ pendingBytes: 0, paused: false }, { pendingBytes: 1, paused: false })).toBe(false)
    })
})
