import { HIGH_WATER_BYTES, LOW_WATER_BYTES } from '@shared/constants/terminal'

export type FlowControlState = {
    pendingBytes: number
    paused: boolean
}

export const INITIAL_FLOW_CONTROL_STATE: FlowControlState = { pendingBytes: 0, paused: false }

export const evaluateFlowControl = (state: FlowControlState, pendingBytes: number): FlowControlState => {
    if (!state.paused && pendingBytes > HIGH_WATER_BYTES) return { pendingBytes, paused: true }
    if (state.paused && pendingBytes < LOW_WATER_BYTES) return { pendingBytes, paused: false }
    return { ...state, pendingBytes }
}

export const shouldTogglePause = (previous: FlowControlState, next: FlowControlState) => previous.paused !== next.paused
