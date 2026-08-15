import { describe, expect, test } from 'bun:test'
import type { IMarker } from '@xterm/xterm'
import {
    INITIAL_OSC133_BLOCK_TRACKER_STATE,
    applyOsc133Event,
    findNextCommandLine,
    findPreviousCommandLine,
    normalizeDecorationHexColor,
    parseOsc133Data,
    parseOsc133ExitCode,
    pruneDisposedBlocks,
    resolveCommandBlockDecorationColor,
} from '@features/terminal/terminal-osc133'
import type { TerminalCommandBlock } from '@features/terminal/terminal-osc133'

const createFakeMarker = (line: number, overrides: Partial<IMarker> = {}): IMarker => ({
    id: 0,
    line,
    isDisposed: false,
    dispose: () => {},
    onDispose: () => ({ dispose: () => {} }),
    ...overrides,
})

describe('parseOsc133Data', () => {
    test('A/B/C/D 종류를 인식한다', () => {
        expect(parseOsc133Data('A')).toEqual({ kind: 'A', params: [] })
        expect(parseOsc133Data('B')).toEqual({ kind: 'B', params: [] })
        expect(parseOsc133Data('C')).toEqual({ kind: 'C', params: [] })
        expect(parseOsc133Data('D;0')).toEqual({ kind: 'D', params: ['0'] })
    })

    test('알 수 없는 종류·확장 파라미터는 unknown 으로 관대하게 처리한다', () => {
        expect(parseOsc133Data('P;k=i')).toEqual({ kind: 'unknown', params: ['k=i'] })
        expect(parseOsc133Data('')).toEqual({ kind: 'unknown', params: [] })
    })

    test('D 뒤에 여러 파라미터가 와도 전부 보존한다', () => {
        expect(parseOsc133Data('D;130;aid=1')).toEqual({ kind: 'D', params: ['130', 'aid=1'] })
    })
})

describe('parseOsc133ExitCode', () => {
    test('정수 문자열을 파싱한다', () => {
        expect(parseOsc133ExitCode(['0'])).toBe(0)
        expect(parseOsc133ExitCode(['127'])).toBe(127)
    })

    test('파라미터가 없으면 null 이다', () => {
        expect(parseOsc133ExitCode([])).toBeNull()
    })

    test('숫자가 아니거나 정수가 아니면 null 이다', () => {
        expect(parseOsc133ExitCode(['abc'])).toBeNull()
        expect(parseOsc133ExitCode(['1.5'])).toBeNull()
    })
})

describe('applyOsc133Event', () => {
    test('A 는 새 블록을 열고 currentBlockIndex 를 가리킨다', () => {
        const marker = createFakeMarker(10)
        const { state, changedBlock } = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'A', params: [] }, () => marker)
        expect(state.blocks).toHaveLength(1)
        expect(state.currentBlockIndex).toBe(0)
        expect(changedBlock).toEqual({ startMarker: marker, outputStartMarker: null, endMarker: null, exitCode: null })
    })

    test('registerMarker 가 undefined 를 반환하면 A 는 상태를 바꾸지 않는다(alt buffer 등 방어)', () => {
        const { state, changedBlock } = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'A', params: [] }, () => undefined)
        expect(state).toBe(INITIAL_OSC133_BLOCK_TRACKER_STATE)
        expect(changedBlock).toBeNull()
    })

    test('C 는 열린 블록의 outputStartMarker 를 채운다', () => {
        const startMarker = createFakeMarker(10)
        const afterA = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'A', params: [] }, () => startMarker)
        const outputMarker = createFakeMarker(11)
        const afterC = applyOsc133Event(afterA.state, { kind: 'C', params: [] }, () => outputMarker)
        expect(afterC.state.blocks[0].outputStartMarker).toBe(outputMarker)
        expect(afterC.changedBlock?.outputStartMarker).toBe(outputMarker)
    })

    test('열린 블록이 없을 때 C 는 no-op 이다', () => {
        const { state, changedBlock } = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'C', params: [] }, () => createFakeMarker(1))
        expect(state).toBe(INITIAL_OSC133_BLOCK_TRACKER_STATE)
        expect(changedBlock).toBeNull()
    })

    test('D 는 블록을 종료하고 exitCode 를 채우며 currentBlockIndex 를 비운다', () => {
        const startMarker = createFakeMarker(10)
        const afterA = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'A', params: [] }, () => startMarker)
        const endMarker = createFakeMarker(15)
        const afterD = applyOsc133Event(afterA.state, { kind: 'D', params: ['0'] }, () => endMarker)
        expect(afterD.state.currentBlockIndex).toBeNull()
        expect(afterD.state.blocks[0]).toEqual({ startMarker, outputStartMarker: null, endMarker, exitCode: 0 })
        expect(afterD.changedBlock?.exitCode).toBe(0)
    })

    test('열린 블록이 없을 때 D 는 no-op 이다', () => {
        const { state, changedBlock } = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'D', params: ['0'] }, () => createFakeMarker(1))
        expect(state).toBe(INITIAL_OSC133_BLOCK_TRACKER_STATE)
        expect(changedBlock).toBeNull()
    })

    test('exitCode 파라미터가 없어도 블록은 종료되고 exitCode 는 null 이다', () => {
        const afterA = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'A', params: [] }, () => createFakeMarker(1))
        const afterD = applyOsc133Event(afterA.state, { kind: 'D', params: [] }, () => createFakeMarker(2))
        expect(afterD.state.currentBlockIndex).toBeNull()
        expect(afterD.state.blocks[0].exitCode).toBeNull()
    })

    test('unknown 종류는 상태를 그대로 반환한다', () => {
        const { state, changedBlock } = applyOsc133Event(INITIAL_OSC133_BLOCK_TRACKER_STATE, { kind: 'unknown', params: [] }, () =>
            createFakeMarker(1),
        )
        expect(state).toBe(INITIAL_OSC133_BLOCK_TRACKER_STATE)
        expect(changedBlock).toBeNull()
    })

    test('A → D → A → D 순서로 블록 2개를 독립적으로 누적한다', () => {
        let state = INITIAL_OSC133_BLOCK_TRACKER_STATE
        state = applyOsc133Event(state, { kind: 'A', params: [] }, () => createFakeMarker(1)).state
        state = applyOsc133Event(state, { kind: 'D', params: ['0'] }, () => createFakeMarker(2)).state
        state = applyOsc133Event(state, { kind: 'A', params: [] }, () => createFakeMarker(10)).state
        state = applyOsc133Event(state, { kind: 'D', params: ['1'] }, () => createFakeMarker(12)).state

        expect(state.blocks).toHaveLength(2)
        expect(state.blocks[0].exitCode).toBe(0)
        expect(state.blocks[1].exitCode).toBe(1)
    })
})

describe('pruneDisposedBlocks', () => {
    const buildBlock = (line: number, isDisposed: boolean): TerminalCommandBlock => ({
        startMarker: createFakeMarker(line, { isDisposed }),
        outputStartMarker: null,
        endMarker: null,
        exitCode: null,
    })

    test('startMarker 가 dispose 된 블록만 제거한다', () => {
        const alive = buildBlock(5, false)
        const dead = buildBlock(1, true)
        const result = pruneDisposedBlocks({ blocks: [dead, alive], currentBlockIndex: null })
        expect(result.blocks).toEqual([alive])
    })

    test('모두 살아있으면 그대로 반환한다', () => {
        const blocks = [buildBlock(1, false), buildBlock(2, false)]
        const result = pruneDisposedBlocks({ blocks, currentBlockIndex: null })
        expect(result.blocks).toEqual(blocks)
    })

    test('열린 블록보다 앞선 블록이 제거되면 currentBlockIndex 가 새 위치로 보정된다', () => {
        const dead = buildBlock(1, true)
        const open = buildBlock(10, false)
        const result = pruneDisposedBlocks({ blocks: [dead, open], currentBlockIndex: 1 })
        expect(result.blocks).toEqual([open])
        expect(result.currentBlockIndex).toBe(0)
    })

    test('열린 블록 자신이 제거되면 currentBlockIndex 는 null 이 된다', () => {
        const open = buildBlock(10, true)
        const result = pruneDisposedBlocks({ blocks: [open], currentBlockIndex: 0 })
        expect(result.blocks).toEqual([])
        expect(result.currentBlockIndex).toBeNull()
    })

    test('열린 블록이 없으면 currentBlockIndex 는 계속 null 이다', () => {
        const dead = buildBlock(1, true)
        const result = pruneDisposedBlocks({ blocks: [dead], currentBlockIndex: null })
        expect(result.currentBlockIndex).toBeNull()
    })
})

describe('findPreviousCommandLine', () => {
    test('현재 줄보다 작은 것 중 가장 가까운(가장 큰) 줄을 찾는다', () => {
        expect(findPreviousCommandLine([1, 5, 10], 12)).toBe(10)
        expect(findPreviousCommandLine([1, 5, 10], 10)).toBe(5)
    })

    test('후보가 없으면 null 이다', () => {
        expect(findPreviousCommandLine([5, 10], 5)).toBeNull()
        expect(findPreviousCommandLine([], 5)).toBeNull()
    })
})

describe('findNextCommandLine', () => {
    test('현재 줄보다 큰 것 중 가장 가까운(가장 작은) 줄을 찾는다', () => {
        expect(findNextCommandLine([1, 5, 10], 3)).toBe(5)
        expect(findNextCommandLine([1, 5, 10], 1)).toBe(5)
    })

    test('후보가 없으면 null 이다', () => {
        expect(findNextCommandLine([1, 5], 5)).toBeNull()
        expect(findNextCommandLine([], 5)).toBeNull()
    })
})

describe('normalizeDecorationHexColor', () => {
    test('6자리 hex 는 그대로 통과한다', () => {
        expect(normalizeDecorationHexColor('#39d353')).toBe('#39d353')
    })

    test('알파 채널이 붙은 8자리 hex 는 6자리로 잘라낸다', () => {
        expect(normalizeDecorationHexColor('#39d353ff')).toBe('#39d353')
    })

    test('null·undefined·빈 문자열은 null 이다', () => {
        expect(normalizeDecorationHexColor(null)).toBeNull()
        expect(normalizeDecorationHexColor(undefined)).toBeNull()
        expect(normalizeDecorationHexColor('')).toBeNull()
    })

    test('rgba()·짧은 hex 등 지원하지 않는 형식은 null 이다', () => {
        expect(normalizeDecorationHexColor('rgba(57, 211, 83, 1)')).toBeNull()
        expect(normalizeDecorationHexColor('#3d5')).toBeNull()
    })
})

describe('resolveCommandBlockDecorationColor', () => {
    const colors = { success: '#39d353', failure: '#f85149' }

    test('exitCode 0 은 success 색상을 고른다', () => {
        expect(resolveCommandBlockDecorationColor(0, colors)).toBe('#39d353')
    })

    test('0 이 아닌 exitCode 는 failure 색상을 고른다', () => {
        expect(resolveCommandBlockDecorationColor(1, colors)).toBe('#f85149')
        expect(resolveCommandBlockDecorationColor(130, colors)).toBe('#f85149')
    })

    test('색상이 테마에 없으면 null 을 그대로 전달해 데코레이션을 생략시킨다', () => {
        expect(resolveCommandBlockDecorationColor(0, { success: null, failure: '#f85149' })).toBeNull()
    })
})
